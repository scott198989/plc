import { useReducer } from "react";

import { verifyLocalFoundation } from "./foundation-client";
import {
  initialFoundationViewState,
  reduceFoundationViewState,
} from "./ui-model";

export const App = (): React.JSX.Element => {
  const [view, dispatch] = useReducer(
    reduceFoundationViewState,
    initialFoundationViewState,
  );

  const verify = async (): Promise<void> => {
    if (view.phase === "loading") {
      return;
    }
    dispatch({ type: "started" });
    try {
      const result = await verifyLocalFoundation();
      dispatch({ type: "succeeded", result });
    } catch (error) {
      dispatch({
        message:
          error instanceof Error
            ? error.message
            : "The local foundation could not be verified.",
        type: "failed",
      });
    }
  };

  const isLoading = view.phase === "loading";
  const successValue = view.phase === "success" ? view.result.value : undefined;

  return (
    <div className="app-shell">
      <header className="site-header">
        <div className="site-header__inner">
          <span aria-hidden="true" className="brand__mark">P1</span>
          <div className="brand__copy">
            <div className="brand__name">PLC Engineering Simulator</div>
          </div>
        </div>
      </header>

      <main className="foundation" id="main-content">
        <p className="foundation__eyebrow">Foundation check</p>
        <h1>Phase 1 technical foundation</h1>
        <p className="foundation__description foundation__description--desktop">
          This phase verifies the local foundation required for the simulator to run
          offline before any PLC features are enabled.
        </p>
        <p className="foundation__description foundation__description--mobile">
          Verify the local technical foundation is ready.
        </p>

        <button
          aria-disabled={isLoading}
          aria-describedby="verification-status"
          className="verify-button"
          onClick={() => void verify()}
          type="button"
        >
          {isLoading ? (
            <span aria-hidden="true" className="verify-button__spinner" />
          ) : (
            <span aria-hidden="true" className="verify-button__check">✓</span>
          )}
          {isLoading ? "Verifying local foundation" : "Verify local foundation"}
        </button>

        <div
          aria-atomic="true"
          aria-live="polite"
          className="verification-status"
          id="verification-status"
          role="status"
        >
          <span className="visually-hidden">
            {view.phase === "initial" && "Foundation check has not run."}
            {view.phase === "loading" && "Foundation check is running."}
            {view.phase === "success" && "Foundation check passed."}
            {view.phase === "error" && `Foundation check failed. ${view.message}`}
          </span>
        </div>

        {view.phase === "error" && (
          <div className="error-callout" role="alert">
            <span aria-hidden="true" className="error-callout__mark">!</span>
            <div>
              <strong>Foundation check unavailable</strong>
              <span>{view.message}</span>
            </div>
          </div>
        )}

        <dl className="result-list" data-phase={view.phase}>
          <div className="result-row">
            <dt>Schema version</dt>
            <dd>{successValue?.schemaVersion ?? "—"}</dd>
          </div>
          <div className="result-row">
            <dt>Build identity</dt>
            <dd>{successValue?.buildIdentity ?? "Not verified"}</dd>
          </div>
          <div className="result-row">
            <dt>Health state</dt>
            <dd className={successValue ? "health-value" : undefined}>
              <span>
                {successValue?.healthState ??
                  (isLoading ? "CHECKING" : "NOT CHECKED")}
              </span>
              {successValue && <span aria-hidden="true" className="health-dot" />}
            </dd>
          </div>
        </dl>

        <aside className="mobile-offline-note">
          <span aria-hidden="true" className="offline-info-mark">i</span>
          <div>
            <strong>Offline by design</strong>
            <span>No PLC features are active in this phase.</span>
          </div>
        </aside>
      </main>

      <footer className="desktop-offline-note">
        <span aria-hidden="true" className="footer-phase-mark">P1</span>
        <span>Offline by design</span>
        <span aria-hidden="true">•</span>
        <span>No PLC features are active in this phase.</span>
      </footer>
    </div>
  );
};
