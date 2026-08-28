#include "bridge_protocol.h"
#include "broker_client.h"

#include <WebView2.h>
#include <WebView2EnvironmentOptions.h>
#include <bcrypt.h>
#include <shlobj.h>
#include <windows.h>
#include <wrl.h>
#include <wrl/event.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <filesystem>
#include <fstream>
#include <iomanip>
#include <optional>
#include <ranges>
#include <sstream>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

namespace govs::shell {
namespace {

using Microsoft::WRL::Callback;
using Microsoft::WRL::ComPtr;
using Microsoft::WRL::Make;

constexpr wchar_t kWindowClass[] = L"GovsPlcWindowsShellV1";
constexpr wchar_t kApplicationUri[] = L"https://govs-plc.local/index.html";
constexpr wchar_t kVirtualHost[] = L"govs-plc.local";
constexpr wchar_t kVerificationArgument[] = L"--verify-native-bridge";
constexpr UINT_PTR kVerificationCloseTimer = 0x5021;

int g_exit_code = 1;

class UserCancelled final : public std::exception {};

std::filesystem::path executable_directory() {
  std::wstring buffer(32'768, L'\0');
  const auto length = GetModuleFileNameW(
      nullptr, buffer.data(), static_cast<DWORD>(buffer.size()));
  if (length == 0 || length >= buffer.size()) {
    throw std::runtime_error("The packaged shell location is unavailable.");
  }
  buffer.resize(length);
  return std::filesystem::path(buffer).parent_path();
}

std::wstring widen_ascii(std::string_view value) {
  return std::wstring(value.begin(), value.end());
}

std::string narrow_ascii(std::wstring_view value) {
  std::string output;
  output.reserve(value.size());
  for (const auto character : value) {
    if (character < 0x20 || character > 0x7e) {
      throw BrokerFailure(
          BrokerErrorCode::invalid_file_name,
          "The native chooser accepts bounded ASCII project names only.");
    }
    output.push_back(static_cast<char>(character));
  }
  return output;
}

std::string bytes_sha256(const std::vector<std::uint8_t>& bytes) {
  BCRYPT_ALG_HANDLE algorithm = nullptr;
  BCRYPT_HASH_HANDLE hash = nullptr;
  DWORD object_bytes = 0;
  DWORD copied = 0;
  if (BCryptOpenAlgorithmProvider(
          &algorithm, BCRYPT_SHA256_ALGORITHM, nullptr, 0) < 0 ||
      BCryptGetProperty(
          algorithm,
          BCRYPT_OBJECT_LENGTH,
          reinterpret_cast<PUCHAR>(&object_bytes),
          sizeof(object_bytes),
          &copied,
          0) < 0 ||
      copied != sizeof(object_bytes)) {
    if (algorithm != nullptr) BCryptCloseAlgorithmProvider(algorithm, 0);
    throw std::runtime_error("SHA-256 initialization failed.");
  }
  std::vector<std::uint8_t> object(object_bytes);
  std::array<std::uint8_t, 32> digest{};
  const bool failed =
      BCryptCreateHash(
          algorithm, &hash, object.data(), object_bytes, nullptr, 0, 0) < 0 ||
      BCryptHashData(
          hash,
          const_cast<PUCHAR>(bytes.data()),
          static_cast<ULONG>(bytes.size()),
          0) < 0 ||
      BCryptFinishHash(
          hash, digest.data(), static_cast<ULONG>(digest.size()), 0) < 0;
  if (hash != nullptr) BCryptDestroyHash(hash);
  BCryptCloseAlgorithmProvider(algorithm, 0);
  if (failed) throw std::runtime_error("SHA-256 hashing failed.");
  std::ostringstream output;
  output << std::uppercase << std::hex << std::setfill('0');
  for (const auto byte : digest) {
    output << std::setw(2) << static_cast<unsigned>(byte);
  }
  return output.str();
}

bool has_canonical_project_header(const std::vector<std::uint8_t>& bytes) {
  constexpr std::array<std::uint8_t, 8> header{
      'V', 'L', 'A', 'B', 'P', 'K', 'G', '1'};
  return bytes.size() >= header.size() &&
         std::ranges::equal(header, bytes | std::views::take(header.size()));
}

std::string verification_project_name(std::uint64_t session_id) {
  std::ostringstream output;
  output << "Phase-2-Native-" << std::hex << std::nouppercase
         << std::setw(16) << std::setfill('0') << session_id << ".vlabproj";
  return output.str();
}

bool is_upper_sha256(std::string_view value) {
  return value.size() == 64 && std::ranges::all_of(value, [](char character) {
    return (character >= '0' && character <= '9') ||
           (character >= 'A' && character <= 'F');
  });
}

std::uint32_t positive_verification_count(std::string_view value) {
  if (value.empty() || value.size() > 7 ||
      (value.size() > 1 && value.front() == '0')) {
    throw std::runtime_error("The verified replay count was not canonical.");
  }
  std::uint32_t result = 0;
  for (const auto character : value) {
    if (character < '0' || character > '9') {
      throw std::runtime_error("The verified replay count was not decimal.");
    }
    result = result * 10 + static_cast<std::uint32_t>(character - '0');
  }
  if (result == 0 || result > 1'000'000) {
    throw std::runtime_error("The verified replay count exceeded its closed bound.");
  }
  return result;
}

class ProjectDialog final {
 public:
  enum class Mode { open, save_as };

  static std::optional<std::string> choose_open(
      HWND owner,
      const std::vector<std::string>& names) {
    if (names.empty()) return std::nullopt;
    ProjectDialog dialog(Mode::open, names, names.front());
    return dialog.run(owner);
  }

  static std::optional<std::string> choose_save_as(
      HWND owner,
      const std::string& suggestion) {
    ProjectDialog dialog(Mode::save_as, {}, suggestion);
    return dialog.run(owner);
  }

 private:
  ProjectDialog(
      Mode mode,
      std::vector<std::string> names,
      std::string suggestion)
      : mode_(mode), names_(std::move(names)), suggestion_(std::move(suggestion)) {}

  std::optional<std::string> run(HWND owner) {
    owner_ = owner;
    static const auto registered = [] {
      WNDCLASSW definition{};
      definition.lpfnWndProc = &ProjectDialog::window_proc;
      definition.hInstance = GetModuleHandleW(nullptr);
      definition.hCursor = LoadCursorW(nullptr, IDC_ARROW);
      definition.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
      definition.lpszClassName = L"GovsPlcTypedProjectChooserV1";
      return RegisterClassW(&definition) != 0 ||
             GetLastError() == ERROR_CLASS_ALREADY_EXISTS;
    }();
    if (!registered) {
      throw std::runtime_error("The typed project chooser could not be registered.");
    }

    const wchar_t* title =
        mode_ == Mode::open ? L"Open local simulator project" : L"Save simulator project as";
    window_ = CreateWindowExW(
        WS_EX_DLGMODALFRAME,
        L"GovsPlcTypedProjectChooserV1",
        title,
        WS_CAPTION | WS_SYSMENU | WS_POPUP,
        CW_USEDEFAULT,
        CW_USEDEFAULT,
        560,
        390,
        owner,
        nullptr,
        GetModuleHandleW(nullptr),
        this);
    if (window_ == nullptr) {
      throw std::runtime_error("The typed project chooser could not be created.");
    }
    RECT owner_bounds{};
    RECT dialog_bounds{};
    GetWindowRect(owner, &owner_bounds);
    GetWindowRect(window_, &dialog_bounds);
    SetWindowPos(
        window_,
        nullptr,
        owner_bounds.left +
            ((owner_bounds.right - owner_bounds.left) -
             (dialog_bounds.right - dialog_bounds.left)) /
                2,
        owner_bounds.top +
            ((owner_bounds.bottom - owner_bounds.top) -
             (dialog_bounds.bottom - dialog_bounds.top)) /
                2,
        0,
        0,
        SWP_NOSIZE | SWP_NOZORDER);
    EnableWindow(owner, FALSE);
    ShowWindow(window_, SW_SHOW);
    UpdateWindow(window_);
    MSG message{};
    while (IsWindow(window_) != 0 && GetMessageW(&message, nullptr, 0, 0) > 0) {
      if (IsDialogMessageW(window_, &message) == 0) {
        TranslateMessage(&message);
        DispatchMessageW(&message);
      }
    }
    EnableWindow(owner, TRUE);
    SetForegroundWindow(owner);
    return result_;
  }

  static LRESULT CALLBACK window_proc(
      HWND window,
      UINT message,
      WPARAM wparam,
      LPARAM lparam) {
    auto* self = reinterpret_cast<ProjectDialog*>(
        GetWindowLongPtrW(window, GWLP_USERDATA));
    if (message == WM_NCCREATE) {
      const auto* create = reinterpret_cast<CREATESTRUCTW*>(lparam);
      self = static_cast<ProjectDialog*>(create->lpCreateParams);
      self->window_ = window;
      SetWindowLongPtrW(window, GWLP_USERDATA, reinterpret_cast<LONG_PTR>(self));
    }
    return self == nullptr ? DefWindowProcW(window, message, wparam, lparam)
                           : self->handle_message(message, wparam, lparam);
  }

  LRESULT handle_message(UINT message, WPARAM wparam, LPARAM lparam) {
    switch (message) {
      case WM_CREATE: {
        CreateWindowExW(
            0,
            L"STATIC",
            mode_ == Mode::open
                ? L"Choose one project already attested inside the fixed local project root."
                : L"Enter one .vlabproj base name. Paths, providers, devices, and remote targets are unavailable.",
            WS_CHILD | WS_VISIBLE,
            24,
            20,
            500,
            42,
            window_,
            nullptr,
            GetModuleHandleW(nullptr),
            nullptr);
        if (mode_ == Mode::open) {
          input_ = CreateWindowExW(
              WS_EX_CLIENTEDGE,
              L"LISTBOX",
              nullptr,
              WS_CHILD | WS_VISIBLE | WS_TABSTOP | LBS_NOTIFY | WS_VSCROLL,
              24,
              70,
              500,
              225,
              window_,
              reinterpret_cast<HMENU>(100),
              GetModuleHandleW(nullptr),
              nullptr);
          for (const auto& name : names_) {
            const auto wide = widen_ascii(name);
            SendMessageW(input_, LB_ADDSTRING, 0, reinterpret_cast<LPARAM>(wide.c_str()));
          }
          SendMessageW(input_, LB_SETCURSEL, 0, 0);
        } else {
          input_ = CreateWindowExW(
              WS_EX_CLIENTEDGE,
              L"EDIT",
              widen_ascii(suggestion_).c_str(),
              WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL,
              24,
              80,
              500,
              30,
              window_,
              reinterpret_cast<HMENU>(100),
              GetModuleHandleW(nullptr),
              nullptr);
          SendMessageW(input_, EM_SETLIMITTEXT, 255, 0);
        }
        CreateWindowExW(
            0,
            L"BUTTON",
            mode_ == Mode::open ? L"Open" : L"Save",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_DEFPUSHBUTTON,
            340,
            315,
            88,
            30,
            window_,
            reinterpret_cast<HMENU>(IDOK),
            GetModuleHandleW(nullptr),
            nullptr);
        CreateWindowExW(
            0,
            L"BUTTON",
            L"Cancel",
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            436,
            315,
            88,
            30,
            window_,
            reinterpret_cast<HMENU>(IDCANCEL),
            GetModuleHandleW(nullptr),
            nullptr);
        SetFocus(input_);
        return 0;
      }
      case WM_COMMAND:
        if (LOWORD(wparam) == IDOK ||
            (mode_ == Mode::open && LOWORD(wparam) == 100 &&
             HIWORD(wparam) == LBN_DBLCLK)) {
          accept();
          return 0;
        }
        if (LOWORD(wparam) == IDCANCEL) {
          DestroyWindow(window_);
          return 0;
        }
        break;
      case WM_CLOSE:
        DestroyWindow(window_);
        return 0;
      default:
        break;
    }
    return DefWindowProcW(window_, message, wparam, lparam);
  }

  void accept() {
    try {
      if (mode_ == Mode::open) {
        const auto selected = SendMessageW(input_, LB_GETCURSEL, 0, 0);
        if (selected == LB_ERR) return;
        result_ = names_.at(static_cast<std::size_t>(selected));
      } else {
        const auto length = GetWindowTextLengthW(input_);
        if (length < 1 || length > 255) return;
        std::wstring value(static_cast<std::size_t>(length) + 1U, L'\0');
        GetWindowTextW(input_, value.data(), length + 1);
        value.resize(length);
        result_ = narrow_ascii(value);
      }
      DestroyWindow(window_);
    } catch (const std::exception&) {
      MessageBoxW(
          window_,
          L"Use one bounded ASCII .vlabproj base name. Paths and host targets are not accepted.",
          L"Invalid project name",
          MB_OK | MB_ICONWARNING);
    }
  }

  Mode mode_;
  std::vector<std::string> names_;
  std::string suggestion_;
  std::optional<std::string> result_;
  HWND owner_{};
  HWND window_{};
  HWND input_{};
};

class ApplicationHost final {
 public:
  ApplicationHost(HWND window, bool verification_mode)
      : window_(window), verification_mode_(verification_mode) {}

  ApplicationHost(const ApplicationHost&) = delete;
  ApplicationHost& operator=(const ApplicationHost&) = delete;

  void initialize() {
    attestation_ = broker_.start();
    if (BCryptGenRandom(
            nullptr,
            reinterpret_cast<PUCHAR>(&session_id_),
            sizeof(session_id_),
            BCRYPT_USE_SYSTEM_PREFERRED_RNG) < 0 ||
        session_id_ == 0) {
      throw std::runtime_error("The native bridge session identity failed closed.");
    }
    verification_project_name_ = verification_project_name(session_id_);

    if (SetEnvironmentVariableW(
            L"WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS", nullptr) == 0 ||
        SetEnvironmentVariableW(
            L"WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", nullptr) == 0 ||
        SetEnvironmentVariableW(L"WEBVIEW2_USER_DATA_FOLDER", nullptr) == 0 ||
        SetEnvironmentVariableW(
            L"WEBVIEW2_RELEASE_CHANNEL_PREFERENCE", nullptr) == 0) {
      throw std::runtime_error(
          "The inherited WebView2 environment could not be cleared.");
    }

    const auto package = executable_directory();
    application_folder_ = package / L"app";
    user_data_folder_ = broker_.prepare_fixed_user_data_folder(session_id_);

    auto options = Make<CoreWebView2EnvironmentOptions>();
    if (!options) throw std::runtime_error("WebView2 options are unavailable.");
    std::wstring browser_arguments =
        L"--disable-background-networking --disable-breakpad "
        L"--disable-component-extensions-with-background-pages "
        L"--disable-component-update --disable-default-apps "
        L"--disable-domain-reliability --disable-logging "
        L"--disable-sync --metrics-recording-only --no-default-browser-check "
        L"--no-first-run --no-pings "
        L"--disable-features=AutofillServerCommunication,CertificateTransparencyComponentUpdater,"
        L"OptimizationHints,MediaRouter,WebRtc,WebRtcHideLocalIpsWithMdns";
    if (verification_mode_) {
      net_log_path_ = package / L"native-netlog.json";
      browser_arguments += L" --log-net-log=\"" + net_log_path_.wstring() +
                           L"\" --net-log-capture-mode=Everything";
    }
    if (FAILED(options->put_AdditionalBrowserArguments(browser_arguments.c_str())) ||
        FAILED(options->put_AllowSingleSignOnUsingOSPrimaryAccount(FALSE)) ||
        FAILED(options->put_ExclusiveUserDataFolderAccess(TRUE)) ||
        FAILED(options->put_IsCustomCrashReportingEnabled(FALSE)) ||
        FAILED(options->put_AreBrowserExtensionsEnabled(FALSE))) {
      throw std::runtime_error("The fail-closed WebView2 options were rejected.");
    }

    const auto status = CreateCoreWebView2EnvironmentWithOptions(
        nullptr,
        user_data_folder_.c_str(),
        options.Get(),
        Callback<ICoreWebView2CreateCoreWebView2EnvironmentCompletedHandler>(
            [this](HRESULT result, ICoreWebView2Environment* environment) -> HRESULT {
              if (FAILED(result) || environment == nullptr) {
                fail_async(L"The approved WebView2 runtime is unavailable.");
                return S_OK;
              }
              environment_ = environment;
              const auto controller_status = environment_->CreateCoreWebView2Controller(
                  window_,
                  Callback<ICoreWebView2CreateCoreWebView2ControllerCompletedHandler>(
                      [this](HRESULT controller_result,
                             ICoreWebView2Controller* controller) -> HRESULT {
                        if (FAILED(controller_result) || controller == nullptr) {
                          fail_async(L"The local WebView2 controller failed closed.");
                          return S_OK;
                        }
                        controller_ = controller;
                        ComPtr<ICoreWebView2Controller4> controller4;
                        if (FAILED(controller_.As(&controller4)) || !controller4 ||
                            FAILED(controller4->put_AllowExternalDrop(FALSE))) {
                          fail_async(L"External file ingress failed closed.");
                          return S_OK;
                        }
                        if (FAILED(controller_->get_CoreWebView2(&webview_)) || !webview_) {
                          fail_async(L"The local WebView2 surface failed closed.");
                          return S_OK;
                        }
                        try {
                          configure_webview();
                        } catch (const std::exception&) {
                          fail_async(L"The local workbench security boundary failed closed.");
                        }
                        return S_OK;
                      })
                      .Get());
              if (FAILED(controller_status)) {
                fail_async(L"The local WebView2 controller could not start.");
              }
              return S_OK;
            })
            .Get());
    if (FAILED(status)) {
      throw std::runtime_error("The approved WebView2 environment failed to start.");
    }
  }

  void resize() const {
    if (controller_) {
      RECT bounds{};
      GetClientRect(window_, &bounds);
      controller_->put_Bounds(bounds);
    }
  }

  [[nodiscard]] bool verification_passed() const noexcept {
    return verification_passed_;
  }

 private:
  void configure_webview() {
    ComPtr<ICoreWebView2Settings> settings;
    if (FAILED(webview_->get_Settings(&settings)) || !settings ||
        FAILED(settings->put_IsScriptEnabled(TRUE)) ||
        FAILED(settings->put_IsWebMessageEnabled(TRUE)) ||
        FAILED(settings->put_AreDefaultScriptDialogsEnabled(FALSE)) ||
        FAILED(settings->put_AreDevToolsEnabled(FALSE)) ||
        FAILED(settings->put_AreDefaultContextMenusEnabled(FALSE)) ||
        FAILED(settings->put_IsStatusBarEnabled(FALSE))) {
      throw std::runtime_error("The WebView2 settings failed closed.");
    }
    ComPtr<ICoreWebView2Settings3> settings3;
    if (FAILED(settings.As(&settings3)) || !settings3 ||
        FAILED(settings3->put_AreBrowserAcceleratorKeysEnabled(FALSE))) {
      throw std::runtime_error("The WebView2 accelerator policy failed closed.");
    }

    ComPtr<ICoreWebView2_3> webview3;
    if (FAILED(webview_.As(&webview3)) || !webview3 ||
        FAILED(webview3->SetVirtualHostNameToFolderMapping(
            kVirtualHost,
            application_folder_.c_str(),
            COREWEBVIEW2_HOST_RESOURCE_ACCESS_KIND_DENY_CORS))) {
      throw std::runtime_error("The fixed local application mapping failed closed.");
    }

    EventRegistrationToken token{};
    auto navigation = Callback<ICoreWebView2NavigationStartingEventHandler>(
        [](ICoreWebView2*, ICoreWebView2NavigationStartingEventArgs* args) -> HRESULT {
          LPWSTR uri = nullptr;
          const bool allowed =
              SUCCEEDED(args->get_Uri(&uri)) && uri != nullptr &&
              std::wstring_view(uri) == kApplicationUri;
          if (uri != nullptr) CoTaskMemFree(uri);
          return args->put_Cancel(allowed ? FALSE : TRUE);
        });
    auto deny_frame_navigation =
        Callback<ICoreWebView2NavigationStartingEventHandler>(
            [](ICoreWebView2*, ICoreWebView2NavigationStartingEventArgs* args)
                -> HRESULT { return args->put_Cancel(TRUE); });
    if (FAILED(webview_->add_NavigationStarting(navigation.Get(), &token)) ||
        FAILED(webview_->add_FrameNavigationStarting(
            deny_frame_navigation.Get(), &token))) {
      throw std::runtime_error("Navigation denial handlers failed closed.");
    }

    if (FAILED(webview_->add_NewWindowRequested(
            Callback<ICoreWebView2NewWindowRequestedEventHandler>(
                [](ICoreWebView2*, ICoreWebView2NewWindowRequestedEventArgs* args) -> HRESULT {
                  return args->put_Handled(TRUE);
                })
                .Get(),
            &token)) ||
        FAILED(webview_->add_PermissionRequested(
            Callback<ICoreWebView2PermissionRequestedEventHandler>(
                [](ICoreWebView2*, ICoreWebView2PermissionRequestedEventArgs* args) -> HRESULT {
                  return args->put_State(COREWEBVIEW2_PERMISSION_STATE_DENY);
                })
                .Get(),
            &token))) {
      throw std::runtime_error("Browser capability denial handlers failed closed.");
    }

    ComPtr<ICoreWebView2_4> webview4;
    if (FAILED(webview_.As(&webview4)) || !webview4 ||
        FAILED(webview4->add_DownloadStarting(
            Callback<ICoreWebView2DownloadStartingEventHandler>(
                [](ICoreWebView2*, ICoreWebView2DownloadStartingEventArgs* args) -> HRESULT {
                  if (FAILED(args->put_Cancel(TRUE))) return E_FAIL;
                  return args->put_Handled(TRUE);
                })
                .Get(),
            &token))) {
      throw std::runtime_error("Download denial failed closed.");
    }

    if (FAILED(webview_->AddWebResourceRequestedFilter(
            L"*", COREWEBVIEW2_WEB_RESOURCE_CONTEXT_ALL)) ||
        FAILED(webview_->add_WebResourceRequested(
            Callback<ICoreWebView2WebResourceRequestedEventHandler>(
                [this](ICoreWebView2*, ICoreWebView2WebResourceRequestedEventArgs* args)
                    -> HRESULT { return on_resource_requested(args); })
                .Get(),
            &token)) ||
        FAILED(webview_->add_WebMessageReceived(
            Callback<ICoreWebView2WebMessageReceivedEventHandler>(
                [this](ICoreWebView2*, ICoreWebView2WebMessageReceivedEventArgs* args)
                    -> HRESULT { return on_web_message(args); })
                .Get(),
            &token))) {
      throw std::runtime_error("The typed local resource/bridge handlers failed closed.");
    }

    const auto bootstrap = bridge_bootstrap_script(
        attestation_.volume_serial, session_id_, verification_mode_);
    if (FAILED(webview_->AddScriptToExecuteOnDocumentCreated(
            bootstrap.c_str(),
            Callback<ICoreWebView2AddScriptToExecuteOnDocumentCreatedCompletedHandler>(
                [this](HRESULT result, LPCWSTR) -> HRESULT {
                  if (FAILED(result) || FAILED(webview_->Navigate(kApplicationUri))) {
                    fail_async(L"The typed native bridge bootstrap failed closed.");
                  }
                  return S_OK;
                })
                .Get()))) {
      throw std::runtime_error("The typed native bridge could not be installed.");
    }
    resize();
  }

  HRESULT on_resource_requested(ICoreWebView2WebResourceRequestedEventArgs* args) {
    ComPtr<ICoreWebView2WebResourceRequest> request;
    LPWSTR raw_uri = nullptr;
    LPWSTR raw_method = nullptr;
    if (FAILED(args->get_Request(&request)) || !request ||
        FAILED(request->get_Uri(&raw_uri)) || raw_uri == nullptr ||
        FAILED(request->get_Method(&raw_method)) || raw_method == nullptr) {
      if (raw_uri != nullptr) CoTaskMemFree(raw_uri);
      if (raw_method != nullptr) CoTaskMemFree(raw_method);
      return E_FAIL;
    }
    const std::wstring_view uri(raw_uri);
    const bool allowed =
        std::wstring_view(raw_method) == L"GET" &&
        (uri == kApplicationUri ||
         uri.starts_with(L"blob:https://govs-plc.local/") ||
         uri.starts_with(L"data:"));
    CoTaskMemFree(raw_uri);
    CoTaskMemFree(raw_method);
    if (allowed) return S_OK;
    ComPtr<ICoreWebView2WebResourceResponse> response;
    if (FAILED(environment_->CreateWebResourceResponse(
            nullptr,
            403,
            L"Blocked",
            L"Content-Type: text/plain; charset=utf-8\r\nCache-Control: no-store",
            &response)) ||
        !response) {
      return E_FAIL;
    }
    return args->put_Response(response.Get());
  }

  HRESULT on_web_message(ICoreWebView2WebMessageReceivedEventArgs* args) {
    LPWSTR raw_source = nullptr;
    const bool admitted_source =
        SUCCEEDED(args->get_Source(&raw_source)) && raw_source != nullptr &&
        std::wstring_view(raw_source) == kApplicationUri;
    if (raw_source != nullptr) CoTaskMemFree(raw_source);
    if (!admitted_source) return S_OK;

    LPWSTR raw = nullptr;
    if (FAILED(args->TryGetWebMessageAsString(&raw)) || raw == nullptr) {
      return S_OK;
    }
    const std::wstring message(raw);
    CoTaskMemFree(raw);
    if (message == L"P2REVKF1") {
      broker_.stop();
      if (verification_mode_) {
        write_verification_manifest("FAIL", "native revoke acknowledgement failed");
      }
      fail_async(L"The native grant revocation acknowledgement failed closed.");
      return S_OK;
    }
    if (verification_mode_ && message.starts_with(L"P2VEFY1|")) {
      try {
        auto remaining = std::wstring_view(message).substr(
            std::wstring_view(L"P2VEFY1|").size());
        std::array<std::string, 4> fields;
        for (std::size_t index = 0; index < fields.size(); ++index) {
          const auto delimiter = remaining.find(L'|');
          if ((index + 1 < fields.size() && delimiter == std::wstring_view::npos) ||
              (index + 1 == fields.size() && delimiter != std::wstring_view::npos)) {
            throw std::runtime_error("The verified replay receipt shape failed closed.");
          }
          fields[index] = narrow_ascii(remaining.substr(0, delimiter));
          if (delimiter != std::wstring_view::npos) {
            remaining.remove_prefix(delimiter + 1);
          }
        }
        const auto event_count = positive_verification_count(fields[2]);
        const auto boundary_count = positive_verification_count(fields[3]);
        if (verification_stage_ != 2 || !is_upper_sha256(fields[0]) ||
            !is_upper_sha256(fields[1]) || !runtime_replay_hash_.empty() ||
            !canonical_replay_hash_.empty() || replay_event_count_ != 0 ||
            replay_boundary_count_ != 0) {
          throw std::runtime_error(
              "The closed deterministic replay receipt failed closed.");
        }
        runtime_replay_hash_ = fields[0];
        canonical_replay_hash_ = fields[1];
        replay_event_count_ = event_count;
        replay_boundary_count_ = boundary_count;
      } catch (const std::exception& error) {
        write_verification_manifest("FAIL", error.what());
        SetTimer(window_, kVerificationCloseTimer, 50, nullptr);
      }
      return S_OK;
    }
    std::uint64_t request_id = 0;
    bool revoke_request = false;
    try {
      const auto request = parse_bridge_request(message);
      request_id = request.request_id;
      revoke_request = request.operation == BridgeOperation::revoke;
      if (request_id <= last_renderer_request_id_) {
        throw BrokerFailure(
            BrokerErrorCode::invalid_frame,
            "The renderer request identity was stale or duplicated.");
      }
      last_renderer_request_id_ = request_id;
      std::wstring response;
      switch (request.operation) {
        case BridgeOperation::open: {
          const auto names = broker_.list_projects();
          std::optional<std::string> selected;
          if (verification_mode_) {
            if (verification_stage_ != 1 ||
                std::ranges::find(names, verification_project_name_) ==
                    names.end()) {
              throw std::runtime_error("Verification open did not observe the created project.");
            }
            selected = verification_project_name_;
          } else {
            selected = ProjectDialog::choose_open(window_, names);
          }
          if (!selected) throw UserCancelled{};
          const auto opened = broker_.open(*selected);
          if (verification_mode_) {
            if (!has_canonical_project_header(opened.bytes) ||
                opened.bytes != verification_created_bytes_) {
              throw std::runtime_error("Verification open bytes were not canonical.");
            }
            verification_grant_ = opened.grant_id;
            ++verification_stage_;
          }
          response = opened_bridge_response(request_id, opened);
          break;
        }
        case BridgeOperation::save_as: {
          std::optional<std::string> selected;
          if (verification_mode_) {
            if (verification_stage_ != 0 ||
                !has_canonical_project_header(request.bytes)) {
              throw std::runtime_error("Verification create input was not canonical.");
            }
            selected = verification_project_name_;
          } else {
            selected = ProjectDialog::choose_save_as(
                window_, request.project_name);
          }
          if (!selected) throw UserCancelled{};
          const auto saved = broker_.save_as(*selected, request.bytes);
          if (verification_mode_) {
            controlled_input_hash_ = bytes_sha256(request.bytes);
            verification_created_bytes_ = request.bytes;
            verification_grant_ = saved.grant_id;
            ++verification_stage_;
          }
          response = saved_bridge_response(request_id, saved);
          break;
        }
        case BridgeOperation::save: {
          if (verification_mode_ &&
               (verification_stage_ != 2 || request.grant_id != verification_grant_ ||
                !has_canonical_project_header(request.bytes) ||
                runtime_replay_hash_.empty() || canonical_replay_hash_.empty() ||
                replay_event_count_ == 0 || replay_boundary_count_ == 0)) {
            throw std::runtime_error("Verification replacement input was not canonical.");
          }
          const auto saved = broker_.save(request.grant_id, request.bytes);
          if (verification_mode_) {
            deterministic_output_hash_ = bytes_sha256(request.bytes);
            verification_grant_ = saved.grant_id;
            ++verification_stage_;
          }
          response = saved_bridge_response(request_id, saved);
          break;
        }
        case BridgeOperation::revoke:
          if (verification_mode_ &&
              (verification_stage_ != 3 || request.grant_id != verification_grant_)) {
            throw std::runtime_error("Verification revoke did not match replacement authority.");
          }
          broker_.revoke(request.grant_id);
          response = revoked_bridge_response(request_id);
          if (verification_mode_) {
            ++verification_stage_;
            verification_passed_ = true;
            write_verification_manifest("PASS", "");
            SetTimer(window_, kVerificationCloseTimer, 250, nullptr);
          }
          break;
      }
      return webview_->PostWebMessageAsString(response.c_str());
    } catch (const UserCancelled&) {
      const auto response = std::wstring(L"P2WEBR1|") + request_hex(request_id) +
                            L"|error|ACCESS_CANCELLED";
      return webview_->PostWebMessageAsString(response.c_str());
    } catch (const BrokerFailure& error) {
      if (revoke_request) {
        // The renderer-facing revoke surface intentionally remains void. A
        // missing native ACK therefore tears down the broker process and the
        // shell instead of allowing retained authority to survive invisibly.
        broker_.stop();
        if (verification_mode_) {
          write_verification_manifest("FAIL", "native revoke was not acknowledged");
        }
        fail_async(L"The native grant revocation failed closed.");
        return S_OK;
      }
      if (request_id != 0) {
        const auto response = error_bridge_response(request_id, error.code());
        return webview_->PostWebMessageAsString(response.c_str());
      }
      return S_OK;
    } catch (const std::exception& error) {
      if (verification_mode_) {
        write_verification_manifest("FAIL", error.what());
        SetTimer(window_, kVerificationCloseTimer, 50, nullptr);
      }
      if (request_id != 0) {
        const auto response = error_bridge_response(
            request_id, BrokerErrorCode::access_unavailable);
        return webview_->PostWebMessageAsString(response.c_str());
      }
      return S_OK;
    }
  }

  static std::wstring request_hex(std::uint64_t request_id) {
    std::wostringstream output;
    output << std::hex << std::nouppercase << std::setw(16) << std::setfill(L'0')
           << request_id;
    return output.str();
  }

  void write_verification_manifest(const char* result, const std::string& error) {
    const auto path = executable_directory() / L"native-run-raw.json";
    std::ofstream output(path, std::ios::binary | std::ios::trunc);
    if (!output) return;
    output << "{\n"
           << "  \"schemaVersion\": \"1.0\",\n"
           << "  \"evidenceKind\": \"WINDOWS_NATIVE_BRIDGE_RAW_RUN\",\n"
           << "  \"result\": \"" << result << "\",\n"
           << "  \"attestationVersion\": 1,\n"
           << "  \"volumeSerial\": " << attestation_.volume_serial << ",\n"
           << "  \"fixedLocalBacking\": true,\n"
           << "  \"providerBacked\": false,\n"
           << "  \"remote\": false,\n"
           << "  \"removable\": false,\n"
           << "  \"special\": false,\n"
           << "  \"redirected\": false,\n"
           << "  \"metadataOnlyBeforeAcceptance\": true,\n"
           << "  \"selectedByteIoBeforeAcceptance\": false,\n"
           << "  \"projectName\": \"" << verification_project_name_ << "\",\n"
           << "  \"verificationJourneyId\": \"govs.native-runnable-hardware-replay/v4\",\n"
           << "  \"verificationUuidVersion\": \"govs-p2-native-verification-uuid-v1\",\n"
           << "  \"verificationUuidSeed\": \"2B42B846-54D0-4C61-9B72-4CD3AFC50001\",\n"
           << "  \"verificationUuidOrdinalStart\": 1,\n"
           << "  \"verificationUuidOrdinalContract\": \"after-saved-document:build=4,power=5,preview=6,commit=7,online=8,run=9,scan=10,stop=11,capture=12\",\n"
           << "  \"controlledInputSha256\": \"" << controlled_input_hash_ << "\",\n"
           << "  \"deterministicOutputSha256\": \"" << deterministic_output_hash_ << "\",\n"
           << "  \"runtimeReplaySha256\": \"" << runtime_replay_hash_ << "\",\n"
           << "  \"canonicalReplaySha256\": \"" << canonical_replay_hash_ << "\",\n"
           << "  \"verifiedReplayEventCount\": " << replay_event_count_ << ",\n"
           << "  \"verifiedReplayBoundaryCount\": " << replay_boundary_count_ << ",\n"
           << "  \"verificationStage\": " << verification_stage_ << ",\n"
           << "  \"operations\": [\"create\", \"open\", \"replace\"],\n"
           << "  \"instrumentationStatus\": \"REQUIRES_EXTERNAL_HARNESS\",\n"
           << "  \"error\": \"";
    for (const auto byte : error) {
      if (byte == '"' || byte == '\\') output << '\\';
      if (byte >= 0x20 && byte <= 0x7e) output << byte;
    }
    output << "\"\n}\n";
  }

  void fail_async(const wchar_t* message) {
    if (verification_mode_) write_verification_manifest("FAIL", "webview initialization");
    MessageBoxW(window_, message, L"PLC Engineering Simulator", MB_OK | MB_ICONERROR);
    SetTimer(window_, kVerificationCloseTimer, 50, nullptr);
  }

  HWND window_{};
  bool verification_mode_{};
  bool verification_passed_{};
  std::uint32_t verification_stage_{};
  std::uint64_t verification_grant_{};
  std::uint64_t session_id_{};
  std::uint64_t last_renderer_request_id_{};
  std::string controlled_input_hash_;
  std::string deterministic_output_hash_;
  std::string runtime_replay_hash_;
  std::string canonical_replay_hash_;
  std::uint32_t replay_event_count_{};
  std::uint32_t replay_boundary_count_{};
  std::string verification_project_name_;
  std::vector<std::uint8_t> verification_created_bytes_;
  std::filesystem::path application_folder_;
  std::filesystem::path user_data_folder_;
  std::filesystem::path net_log_path_;
  BrokerAttestation attestation_{};
  BrokerClient broker_;
  ComPtr<ICoreWebView2Environment> environment_;
  ComPtr<ICoreWebView2Controller> controller_;
  ComPtr<ICoreWebView2> webview_;
};

ApplicationHost* g_host = nullptr;

LRESULT CALLBACK application_window_proc(
    HWND window,
    UINT message,
    WPARAM wparam,
    LPARAM lparam) {
  switch (message) {
    case WM_SIZE:
      if (g_host != nullptr) g_host->resize();
      return 0;
    case WM_TIMER:
      if (wparam == kVerificationCloseTimer) {
        KillTimer(window, kVerificationCloseTimer);
        g_exit_code = g_host != nullptr && g_host->verification_passed() ? 0 : 1;
        DestroyWindow(window);
        return 0;
      }
      break;
    case WM_DESTROY:
      PostQuitMessage(0);
      return 0;
    default:
      break;
  }
  return DefWindowProcW(window, message, wparam, lparam);
}

bool is_verification_mode(PWSTR command_line) {
  const std::wstring value = command_line == nullptr ? L"" : command_line;
  if (value.empty()) return false;
  if (value == kVerificationArgument) return true;
  throw std::runtime_error("The Windows shell accepts no arbitrary command-line arguments.");
}

}  // namespace
}  // namespace govs::shell

int WINAPI wWinMain(HINSTANCE instance, HINSTANCE, PWSTR command_line, int show_command) {
  using namespace govs::shell;
  bool verification_mode = false;
  try {
    verification_mode = is_verification_mode(command_line);
  } catch (const std::exception&) {
    return 2;
  }
  if (FAILED(CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED))) return 3;
  SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

  WNDCLASSW definition{};
  definition.lpfnWndProc = application_window_proc;
  definition.hInstance = instance;
  definition.hCursor = LoadCursorW(nullptr, IDC_ARROW);
  definition.hbrBackground = reinterpret_cast<HBRUSH>(COLOR_WINDOW + 1);
  definition.lpszClassName = kWindowClass;
  if (RegisterClassW(&definition) == 0) {
    CoUninitialize();
    return 4;
  }
  const auto window = CreateWindowExW(
      0,
      kWindowClass,
      L"PLC Engineering Simulator",
      WS_OVERLAPPEDWINDOW,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      1500,
      920,
      nullptr,
      nullptr,
      instance,
      nullptr);
  if (window == nullptr) {
    CoUninitialize();
    return 5;
  }

  ApplicationHost host(window, verification_mode);
  g_host = &host;
  try {
    host.initialize();
  } catch (const std::exception&) {
    MessageBoxW(
        window,
        L"The fixed-local Windows shell could not establish its approved native boundary.",
        L"PLC Engineering Simulator",
        MB_OK | MB_ICONERROR);
    DestroyWindow(window);
  }
  if (!verification_mode) {
    ShowWindow(window, show_command);
    UpdateWindow(window);
  }
  MSG message{};
  while (GetMessageW(&message, nullptr, 0, 0) > 0) {
    TranslateMessage(&message);
    DispatchMessageW(&message);
  }
  g_host = nullptr;
  CoUninitialize();
  return verification_mode ? g_exit_code : 0;
}
