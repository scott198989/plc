import { useId, useState } from "react";

import { verifyLocalFoundation } from "./foundation-client";
import { TutorialLaunchButton } from "./GuidedTutorial";
import { ThemeToggle } from "./ThemeToggle";
import type { AppTheme } from "./ThemeToggle";

type ProjectHomeProps = Readonly<{
  busy: boolean;
  coreLabel: string | null;
  error: string | null;
  fileAccessAvailable: boolean;
  onCreate: (displayName: string) => Promise<void>;
  onOpen: () => Promise<void>;
  onStartTutorial: () => void;
  onToggleTheme: () => void;
  theme: AppTheme;
}>;

export const ProjectHome = ({
  busy,
  coreLabel,
  error,
  fileAccessAvailable,
  onCreate,
  onOpen,
  onStartTutorial,
  onToggleTheme,
  theme,
}: ProjectHomeProps): React.JSX.Element => {
  const nameInputId = useId();
  const [name, setName] = useState("My PLC Lab");
  const [foundation, setFoundation] = useState<Readonly<{
    buildIdentity: string;
    healthState: "HEALTHY";
    schemaVersion: number;
  }> | null>(null);
  const [foundationBusy, setFoundationBusy] = useState(false);

  return (
    <div className="home-shell">
      <header className="home-header">
        <div className="product-mark" aria-hidden="true">
          <span>VL</span>
        </div>
        <div>
          <p className="product-kicker">Offline engineering workspace</p>
          <div className="product-name">PLC Engineering Simulator</div>
        </div>
        <div className="core-state" data-ready={coreLabel !== null}>
          <span className="core-state__dot" aria-hidden="true" />
          {coreLabel === null ? "Starting core" : `Core ${coreLabel}`}
        </div>
        <TutorialLaunchButton onClick={onStartTutorial} />
        <ThemeToggle onToggle={onToggleTheme} theme={theme} />
      </header>

      <main className="home-main" id="main-content">
        <section className="home-intro" aria-labelledby="home-title">
          <p className="eyebrow">Virtual PLC Lab</p>
          <h1 id="home-title">Build logic. Test it safely. Understand every scan.</h1>
          <p>
            Create an original virtual controller project, engineer it locally, and keep the
            entire workflow inside this application.
          </p>
          <div className="safety-strip">
            <span className="safety-strip__icon" aria-hidden="true">◇</span>
            <div>
              <strong>Virtual systems only</strong>
              <span>No connection to physical controllers or industrial networks.</span>
            </div>
          </div>
        </section>

        <section className="project-actions" aria-label="Project actions">
          <article className="project-action-card project-action-card--primary" data-tutorial-target="create-project">
            <span className="action-number" aria-hidden="true">01</span>
            <div className="action-copy">
              <p className="action-kicker">Start clean</p>
              <h2>New project</h2>
              <p>Create a simulator-native project with stable identity and local persistence.</p>
            </div>
            <form
              className="new-project-form"
              onSubmit={(event) => {
                event.preventDefault();
                const normalized = name.trim();
                if (normalized.length > 0 && !busy && coreLabel !== null) {
                  void onCreate(normalized);
                }
              }}
            >
              <label htmlFor={nameInputId}>Project name</label>
              <div className="field-with-button">
                <input
                  autoComplete="off"
                  disabled={busy}
                  id={nameInputId}
                  maxLength={128}
                  onChange={(event) => setName(event.target.value)}
                  spellCheck="false"
                  value={name}
                />
                <button
                  className="primary-button"
                  disabled={busy || coreLabel === null || name.trim().length === 0}
                  type="submit"
                >
                  Create
                  <span aria-hidden="true">→</span>
                </button>
              </div>
            </form>
          </article>

          <article className="project-action-card">
            <span className="action-number" aria-hidden="true">02</span>
            <div className="action-copy">
              <p className="action-kicker">Continue locally</p>
              <h2>Open project</h2>
              <p>Choose a <code>.vlabproj</code> file. The file grant stays in this session.</p>
            </div>
            <button
              className="secondary-button"
              disabled={busy || coreLabel === null || !fileAccessAvailable}
              onClick={() => void onOpen()}
              type="button"
            >
              <span aria-hidden="true">↗</span>
              Choose project file
            </button>
            {!fileAccessAvailable && (
              <p className="field-note" role="note">
                Local file grants are unavailable in this browser. You can still create and
                exercise a project in the current session.
              </p>
            )}
          </article>
        </section>

        {error !== null && (
          <div className="home-error" role="alert">
            <strong>Action not completed</strong>
            <span>{error}</span>
          </div>
        )}
      </main>

      <footer className="home-footer">
        <div className="home-footer__principles">
          <span>Local-first</span>
          <span aria-hidden="true">·</span>
          <span>Deterministic core</span>
          <span aria-hidden="true">·</span>
          <span>Original virtual hardware</span>
        </div>
        <div className="foundation-compatibility">
          <button
            disabled={foundationBusy}
            onClick={() => {
              setFoundationBusy(true);
              void verifyLocalFoundation().then(
                (result) => setFoundation(result.value),
                () => setFoundation(null),
              ).finally(() => setFoundationBusy(false));
            }}
            type="button"
          >
            {foundationBusy ? "Verifying local foundation" : "Verify local foundation"}
          </button>
          <dl className="result-list" aria-label="Foundation compatibility result">
            <div><dt>Schema</dt><dd>{foundation?.schemaVersion ?? "—"}</dd></div>
            <div><dt>Build</dt><dd>{foundation?.buildIdentity ?? "Not verified"}</dd></div>
            <div><dt>Health</dt><dd>{foundation?.healthState ?? "NOT CHECKED"}</dd></div>
          </dl>
        </div>
      </footer>
    </div>
  );
};
