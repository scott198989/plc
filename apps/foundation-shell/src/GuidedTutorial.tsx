import { useEffect, useId, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { createPortal } from "react-dom";

import {
  guidedTutorialDefinitions,
  guidedTutorialProgress,
} from "./guided-tutorial";
import type { GuidedTutorialStep } from "./guided-tutorial";

type SpotlightRect = Readonly<{
  bottom: number;
  height: number;
  left: number;
  right: number;
  top: number;
  width: number;
}>;

type CardPlacement = "above" | "below" | "left" | "right";

type CardSize = Readonly<{
  height: number;
  width: number;
}>;

type CardLayout = Readonly<{
  arrow: string;
  arrowLeft: number;
  arrowTop: number;
  cardLeft: number;
  cardTop: number;
  placement: CardPlacement;
}>;

type ViewportSize = Readonly<{
  height: number;
  width: number;
}>;

type AccessibilityIsolationSnapshot = Readonly<{
  ariaHidden: string | null;
  inert: boolean;
  inertAttribute: string | null;
}>;

type AccessibilityIsolationBranches = Readonly<{
  exposed: ReadonlySet<HTMLElement>;
  hidden: ReadonlySet<HTMLElement>;
}>;

export type GuidedTutorialProps = Readonly<{
  onAdvance: () => void;
  onExit: () => void;
  onFinish: () => void;
  onReview: () => void;
  step: GuidedTutorialStep;
}>;

export const TutorialLaunchButton = ({
  compact = false,
  onClick,
}: Readonly<{
  compact?: boolean;
  onClick: () => void;
}>): React.JSX.Element => (
  <button
    aria-label="Open ladder tutorial"
    className={`tutorial-launch${compact ? " tutorial-launch--compact" : ""}`}
    onClick={onClick}
    title="Open the first ladder program tutorial"
    type="button"
  >
    <span aria-hidden="true">?</span>
    {!compact && <span>Tutorial</span>}
  </button>
);

const targetPadding = 10;
const cardGap = 26;
const cardMargin = 16;

export const GuidedTutorial = ({
  onAdvance,
  onExit,
  onFinish,
  onReview,
  step,
}: GuidedTutorialProps): React.JSX.Element => {
  const definition = guidedTutorialDefinitions[step];
  const progress = guidedTutorialProgress(step);
  const instructionId = useId();
  const cardRef = useRef<HTMLElement | null>(null);
  const tutorialRootRef = useRef<HTMLDivElement | null>(null);
  const previousFocusRef = useRef<HTMLElement | null>(null);
  const focusSessionRef = useRef(0);
  const advancedRef = useRef(false);
  const [target, setTarget] = useState<HTMLElement | null>(null);
  const [rect, setRect] = useState<SpotlightRect | null>(null);
  const [cardSize, setCardSize] = useState<CardSize | null>(null);
  const [viewportSize, setViewportSize] = useState<ViewportSize>(readViewportSize);
  const [nudge, setNudge] = useState(0);

  useEffect(() => {
    const focusSession = focusSessionRef.current + 1;
    focusSessionRef.current = focusSession;
    previousFocusRef.current = document.activeElement instanceof HTMLElement
      ? document.activeElement
      : null;
    return () => {
      const previous = previousFocusRef.current;
      window.requestAnimationFrame(() => {
        if (
          focusSessionRef.current === focusSession
          && document.querySelector(".guided-tutorial") === null
          && previous?.isConnected === true
        ) {
          previous.focus({ preventScroll: true });
        }
      });
    };
  }, []);

  useEffect(() => {
    const card = cardRef.current;
    if (card === null) {
      return;
    }

    let frame = 0;
    const measure = (): void => {
      frame = 0;
      const bounds = card.getBoundingClientRect();
      const next = { height: bounds.height, width: bounds.width };
      setCardSize((current) => current?.height === next.height && current.width === next.width
        ? current
        : next);
    };
    const scheduleMeasure = (): void => {
      if (frame === 0) {
        frame = window.requestAnimationFrame(measure);
      }
    };
    const resizeObserver = new ResizeObserver(scheduleMeasure);
    resizeObserver.observe(card);
    measure();

    return () => {
      if (frame !== 0) {
        window.cancelAnimationFrame(frame);
      }
      resizeObserver.disconnect();
    };
  }, []);

  useEffect(() => {
    const updateViewportSize = (): void => setViewportSize(readViewportSize());
    window.addEventListener("resize", updateViewportSize);
    window.visualViewport?.addEventListener("resize", updateViewportSize);
    updateViewportSize();
    return () => {
      window.removeEventListener("resize", updateViewportSize);
      window.visualViewport?.removeEventListener("resize", updateViewportSize);
    };
  }, []);

  useEffect(() => {
    advancedRef.current = false;
    setTarget(null);
    setRect(null);
    if (definition.target === null) {
      return;
    }

    let observed: HTMLElement | null = null;
    let frame = 0;
    let scrolled = false;
    const resizeObserver = new ResizeObserver(() => scheduleMeasure());

    const measure = (): void => {
      frame = 0;
      if (observed === null || !observed.isConnected) {
        locate();
        return;
      }
      const bounds = observed.getBoundingClientRect();
      if (bounds.width <= 0 || bounds.height <= 0) {
        setRect(null);
        return;
      }
      const viewport = readViewportSize();
      const left = clamp(bounds.left - targetPadding, 0, viewport.width);
      const top = clamp(bounds.top - targetPadding, 0, viewport.height);
      const right = clamp(bounds.right + targetPadding, 0, viewport.width);
      const bottom = clamp(bounds.bottom + targetPadding, 0, viewport.height);
      setRect({ bottom, height: bottom - top, left, right, top, width: right - left });
    };

    const scheduleMeasure = (): void => {
      if (frame === 0) {
        frame = window.requestAnimationFrame(measure);
      }
    };

    const locate = (): void => {
      const next = document.querySelector<HTMLElement>(
        `[data-tutorial-target="${definition.target}"]`,
      );
      if (next !== observed) {
        resizeObserver.disconnect();
        observed = next;
        setTarget(next);
        scrolled = false;
        if (next !== null) {
          resizeObserver.observe(next);
        }
      }
      if (observed !== null && !scrolled) {
        scrolled = true;
        observed.scrollIntoView({ behavior: "smooth", block: "center", inline: "center" });
      }
      scheduleMeasure();
    };

    const mutationObserver = new MutationObserver(locate);
    mutationObserver.observe(document.body, { childList: true, subtree: true });
    window.addEventListener("resize", scheduleMeasure);
    window.addEventListener("scroll", scheduleMeasure, true);
    window.visualViewport?.addEventListener("resize", scheduleMeasure);
    window.visualViewport?.addEventListener("scroll", scheduleMeasure);
    locate();

    return () => {
      if (frame !== 0) {
        window.cancelAnimationFrame(frame);
      }
      resizeObserver.disconnect();
      mutationObserver.disconnect();
      window.removeEventListener("resize", scheduleMeasure);
      window.removeEventListener("scroll", scheduleMeasure, true);
      window.visualViewport?.removeEventListener("resize", scheduleMeasure);
      window.visualViewport?.removeEventListener("scroll", scheduleMeasure);
    };
  }, [definition.target, step]);

  useEffect(() => {
    const targetControl = getInitialTargetControl(target);
    if (targetControl === null) {
      return;
    }
    const priorDescription = targetControl.getAttribute("aria-describedby");
    const descriptions = new Set(priorDescription?.split(/\s+/).filter(Boolean) ?? []);
    descriptions.add(instructionId);
    targetControl.setAttribute(
      "aria-describedby",
      [...descriptions].join(" "),
    );
    return () => {
      if (priorDescription === null) {
        targetControl.removeAttribute("aria-describedby");
      } else {
        targetControl.setAttribute("aria-describedby", priorDescription);
      }
    };
  }, [instructionId, target]);

  useEffect(() => {
    const tutorialRoot = tutorialRootRef.current;
    if (tutorialRoot === null) {
      return;
    }

    const snapshots = new Map<HTMLElement, AccessibilityIsolationSnapshot>();

    const remember = (element: HTMLElement): void => {
      if (!snapshots.has(element)) {
        snapshots.set(element, readAccessibilityIsolationSnapshot(element));
      }
    };
    const expose = (element: HTMLElement): void => {
      remember(element);
      element.inert = false;
      element.removeAttribute("inert");
      if (element.getAttribute("aria-hidden")?.toLocaleLowerCase("en-US") === "true") {
        element.removeAttribute("aria-hidden");
      }
    };
    const hide = (element: HTMLElement): void => {
      remember(element);
      element.inert = true;
      element.setAttribute("aria-hidden", "true");
    };
    const restore = (element: HTMLElement): void => {
      const snapshot = snapshots.get(element);
      if (snapshot === undefined) {
        return;
      }
      restoreAccessibilityIsolationSnapshot(element, snapshot);
      snapshots.delete(element);
    };
    const refreshIsolation = (): void => {
      const branches = collectAccessibilityIsolationBranches(tutorialRoot, target);
      const managed = new Set([...branches.exposed, ...branches.hidden]);

      for (const element of [...snapshots.keys()]) {
        if (!managed.has(element)) {
          restore(element);
        }
      }
      for (const element of branches.exposed) {
        expose(element);
      }

      const activeElement = document.activeElement;
      const focusIsAllowed = activeElement instanceof Node && (
        tutorialRoot.contains(activeElement) || target?.contains(activeElement) === true
      );
      if (!focusIsAllowed) {
        (getInitialTargetControl(target) ?? cardRef.current)?.focus({ preventScroll: true });
      }

      for (const element of branches.hidden) {
        hide(element);
      }
    };

    const mutationObserver = new MutationObserver(refreshIsolation);
    mutationObserver.observe(document.body, { childList: true, subtree: true });
    refreshIsolation();

    return () => {
      mutationObserver.disconnect();
      for (const [element, snapshot] of snapshots) {
        restoreAccessibilityIsolationSnapshot(element, snapshot);
      }
      snapshots.clear();
    };
  }, [target]);

  useEffect(() => {
    if (definition.advanceOnTargetClick !== true || target === null) {
      return;
    }
    let advanceTimer: number | null = null;
    const advanceAfterAction = (): void => {
      if (advancedRef.current || target.matches(":disabled, [aria-disabled='true']")) {
        return;
      }
      advancedRef.current = true;
      advanceTimer = window.setTimeout(onAdvance, 220);
    };
    target.addEventListener("click", advanceAfterAction);
    return () => {
      target.removeEventListener("click", advanceAfterAction);
      if (advanceTimer !== null) {
        window.clearTimeout(advanceTimer);
      }
    };
  }, [definition.advanceOnTargetClick, onAdvance, target]);

  useEffect(() => {
    const focusTutorialTarget = (): void => {
      (getInitialTargetControl(target) ?? cardRef.current)?.focus();
    };
    const frame = window.requestAnimationFrame(focusTutorialTarget);
    return () => window.cancelAnimationFrame(frame);
  }, [step, target]);

  useEffect(() => {
    const allowed = (node: EventTarget | null): boolean =>
      node instanceof Node && (
        target?.contains(node) === true || cardRef.current?.contains(node) === true
      );
    const blockOutsidePointer = (event: Event): void => {
      if (allowed(event.target)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      setNudge((current) => current + 1);
    };
    const keepFocusInside = (event: FocusEvent): void => {
      if (allowed(event.target)) {
        return;
      }
      (getInitialTargetControl(target) ?? cardRef.current)?.focus();
    };
    const handleKeys = (event: KeyboardEvent): void => {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onExit();
        return;
      }
      const shortcutKey = event.key.toLocaleLowerCase("en-US");
      const commandKey = event.ctrlKey || event.metaKey;
      const editing = isTextEditingTarget(event.target);
      if (
        (commandKey && shortcutKey === "s")
        || (commandKey && event.shiftKey && shortcutKey === "f")
        || (!editing && commandKey && (shortcutKey === "z" || shortcutKey === "y"))
        || (!editing && event.key === "Delete")
      ) {
        event.preventDefault();
      }
      if (event.key !== "Tab") {
        return;
      }
      const allowedFocus = [
        ...getTargetControls(target),
        ...(cardRef.current === null
          ? []
          : Array.from(cardRef.current.querySelectorAll<HTMLElement>(focusableSelector))),
      ].filter((element) => !element.matches(":disabled, [aria-disabled='true']"));
      if (allowedFocus.length === 0) {
        return;
      }
      const current = allowedFocus.indexOf(document.activeElement as HTMLElement);
      const next = event.shiftKey
        ? current <= 0 ? allowedFocus.length - 1 : current - 1
        : current < 0 || current === allowedFocus.length - 1 ? 0 : current + 1;
      event.preventDefault();
      allowedFocus[next]?.focus();
    };
    // Let the highlighted control and tutorial card process the keystroke first,
    // then prevent workbench-level shortcuts registered on window from seeing it.
    // Native field undo/redo and ordinary typing remain available.
    const suppressApplicationKeydown = (event: KeyboardEvent): void => {
      const key = event.key.toLocaleLowerCase("en-US");
      const editing = event.target instanceof HTMLInputElement ||
        event.target instanceof HTMLTextAreaElement ||
        event.target instanceof HTMLSelectElement ||
        (event.target instanceof HTMLElement && event.target.isContentEditable);
      const applicationShortcut = (event.ctrlKey || event.metaKey) && (
        key === "s" ||
        key === "y" ||
        key === "z" ||
        (event.shiftKey && key === "f")
      );
      if (applicationShortcut && !(editing && (key === "y" || key === "z"))) {
        event.preventDefault();
      }
      event.stopPropagation();
    };

    document.addEventListener("pointerdown", blockOutsidePointer, true);
    document.addEventListener("click", blockOutsidePointer, true);
    document.addEventListener("focusin", keepFocusInside, true);
    document.addEventListener("keydown", handleKeys, true);
    document.addEventListener("keydown", suppressApplicationKeydown);
    return () => {
      document.removeEventListener("pointerdown", blockOutsidePointer, true);
      document.removeEventListener("click", blockOutsidePointer, true);
      document.removeEventListener("focusin", keepFocusInside, true);
      document.removeEventListener("keydown", handleKeys, true);
      document.removeEventListener("keydown", suppressApplicationKeydown);
    };
  }, [onExit, target]);

  const preferredCardWidth = Math.max(
    0,
    Math.min(definition.target === null ? 370 : 360, viewportSize.width - cardMargin * 2),
  );
  const cardMaxHeight = Math.max(0, viewportSize.height - cardMargin * 2);
  const layout = rect === null || cardSize === null
    ? null
    : placeCard(rect, cardSize, viewportSize);
  const cardStyle: CSSProperties = layout === null
    ? {
        left: "50%",
        maxHeight: cardMaxHeight,
        top: "50%",
        transform: "translate(-50%, -50%)",
        width: preferredCardWidth,
      }
    : {
        left: layout.cardLeft,
        maxHeight: cardMaxHeight,
        top: layout.cardTop,
        width: preferredCardWidth,
      };

  return createPortal(
    <div
      aria-label="Guided ladder tutorial"
      className="guided-tutorial"
      data-step={step}
      ref={tutorialRootRef}
    >
      {rect === null ? (
        <div className="guided-tutorial__scrim guided-tutorial__scrim--full" />
      ) : (
        <>
          <div className="guided-tutorial__scrim" style={{ height: rect.top, inset: "0 0 auto" }} />
          <div className="guided-tutorial__scrim" style={{ height: rect.height, left: 0, top: rect.top, width: rect.left }} />
          <div className="guided-tutorial__scrim" style={{ height: rect.height, left: rect.right, right: 0, top: rect.top }} />
          <div className="guided-tutorial__scrim" style={{ bottom: 0, left: 0, right: 0, top: rect.bottom }} />
          <div
            aria-hidden="true"
            className="guided-tutorial__ring"
            style={{ height: rect.height, left: rect.left, top: rect.top, width: rect.width }}
          />
          {layout !== null && (
            <span
              aria-hidden="true"
              className="guided-tutorial__pointer"
              data-placement={layout.placement}
              style={{ left: layout.arrowLeft, top: layout.arrowTop }}
            >{layout.arrow}</span>
          )}
        </>
      )}

      <section
        aria-describedby={instructionId}
        aria-labelledby={`${instructionId}-title`}
        aria-modal={target === null ? "true" : undefined}
        className="guided-tutorial__card"
        data-nudge={nudge % 2}
        ref={cardRef}
        role="dialog"
        style={cardStyle}
        tabIndex={-1}
      >
        <header>
          <div>
            <span>{step === "review" ? "Tutorial review" : step === "complete" ? "Lesson complete" : "First ladder lesson"}</span>
            <h2 id={`${instructionId}-title`}>{definition.title}</h2>
          </div>
          {progress !== null && <strong>{progress.current}/{progress.total}</strong>}
        </header>
        <p id={instructionId}>{definition.body}</p>

        {step === "review" && (
          <ol className="guided-tutorial__review">
            <li><strong>Stop_PB</strong><span>Normally closed and in series; it can interrupt every powered path.</span></li>
            <li><strong>Start_PB</strong><span>Normally open; pressing it first energizes Motor_Run.</span></li>
            <li><strong>Motor_Run contact</strong><span>Parallel with Start_PB; it holds the rung on after Start is released.</span></li>
            <li><strong>Motor_Run coil</strong><span>The output that drives the virtual actuator and its own holding contact.</span></li>
          </ol>
        )}

        {definition.tip !== null && <div className="guided-tutorial__tip"><span aria-hidden="true">i</span>{definition.tip}</div>}
        {definition.target !== null && target === null && (
          <p className="guided-tutorial__locating" role="status">Preparing the next highlighted control…</p>
        )}
        {definition.target !== null && target !== null && (
          <p className="guided-tutorial__instruction" role="status">
            <span aria-hidden="true">➜</span> Use the highlighted control to continue.
          </p>
        )}

        <footer>
          {step === "complete" ? (
            <>
              <button className="guided-tutorial__secondary" onClick={onReview} type="button">Review what you built</button>
              <button className="guided-tutorial__primary" onClick={onFinish} type="button">Finish tutorial</button>
            </>
          ) : step === "review" ? (
            <button className="guided-tutorial__primary" onClick={onFinish} type="button">Close review</button>
          ) : (
            <>
              <small>Esc exits · Tab stays inside this step</small>
              <button className="guided-tutorial__secondary" onClick={onExit} type="button">Skip tutorial</button>
            </>
          )}
        </footer>
      </section>
    </div>,
    document.body,
  );
};

const focusableSelector = [
  "button:not(:disabled)",
  "input:not(:disabled)",
  "select:not(:disabled)",
  "textarea:not(:disabled)",
  "[href]",
  "[tabindex]:not([tabindex='-1'])",
].join(",");

const getTargetControls = (target: HTMLElement | null): HTMLElement[] => target === null
  ? []
  : target.matches(focusableSelector)
    ? [target]
    : Array.from(target.querySelectorAll<HTMLElement>(focusableSelector));

const getInitialTargetControl = (target: HTMLElement | null): HTMLElement | null =>
  getTargetControls(target)[0] ?? null;

const isTextEditingTarget = (target: EventTarget | null): boolean =>
  target instanceof HTMLInputElement
  || target instanceof HTMLTextAreaElement
  || target instanceof HTMLSelectElement
  || (target instanceof HTMLElement && (
    target.isContentEditable || target.closest("[contenteditable='true']") !== null
  ));

const collectAccessibilityIsolationBranches = (
  tutorialRoot: HTMLElement,
  target: HTMLElement | null,
): AccessibilityIsolationBranches => {
  const exposed = new Set<HTMLElement>();
  const hidden = new Set<HTMLElement>();
  const allowedRoots = target === null || !target.isConnected
    ? [tutorialRoot]
    : [tutorialRoot, target];

  const visit = (parent: HTMLElement): void => {
    for (const child of Array.from(parent.children)) {
      if (!(child instanceof HTMLElement)) {
        continue;
      }
      const containsAllowedRoot = allowedRoots.some(
        (allowedRoot) => child === allowedRoot || child.contains(allowedRoot),
      );
      if (!containsAllowedRoot) {
        hidden.add(child);
        continue;
      }

      exposed.add(child);
      if (!allowedRoots.includes(child)) {
        visit(child);
      }
    }
  };

  visit(document.body);
  return { exposed, hidden };
};

const readAccessibilityIsolationSnapshot = (
  element: HTMLElement,
): AccessibilityIsolationSnapshot => ({
  ariaHidden: element.getAttribute("aria-hidden"),
  inert: element.inert,
  inertAttribute: element.getAttribute("inert"),
});

const restoreAccessibilityIsolationSnapshot = (
  element: HTMLElement,
  snapshot: AccessibilityIsolationSnapshot,
): void => {
  element.inert = snapshot.inert;
  if (snapshot.inertAttribute === null) {
    element.removeAttribute("inert");
  } else {
    element.setAttribute("inert", snapshot.inertAttribute);
  }
  if (snapshot.ariaHidden === null) {
    element.removeAttribute("aria-hidden");
  } else {
    element.setAttribute("aria-hidden", snapshot.ariaHidden);
  }
};

const readViewportSize = (): ViewportSize => ({
  height: window.visualViewport?.height ?? window.innerHeight,
  width: window.visualViewport?.width ?? window.innerWidth,
});

const placeCard = (
  rect: SpotlightRect,
  cardSize: CardSize,
  viewport: ViewportSize,
): CardLayout => {
  const cardWidth = Math.min(cardSize.width, viewport.width - cardMargin * 2);
  const cardHeight = Math.min(cardSize.height, viewport.height - cardMargin * 2);
  const room = {
    above: rect.top,
    below: viewport.height - rect.bottom,
    left: rect.left,
    right: viewport.width - rect.right,
  };
  const placement: CardPlacement = room.right >= cardWidth + cardGap
    ? "right"
    : room.left >= cardWidth + cardGap
      ? "left"
      : room.below >= cardHeight + cardGap
        ? "below"
        : "above";

  if (placement === "right" || placement === "left") {
    const cardLeft = placement === "right"
      ? rect.right + cardGap
      : rect.left - cardGap - cardWidth;
    return {
      arrow: placement === "right" ? "←" : "→",
      arrowLeft: placement === "right" ? rect.right + 5 : rect.left - 39,
      arrowTop: clamp(rect.top + rect.height / 2 - 18, 8, viewport.height - 44),
      cardLeft: clamp(cardLeft, cardMargin, viewport.width - cardWidth - cardMargin),
      cardTop: clamp(
        rect.top + rect.height / 2 - cardHeight / 2,
        cardMargin,
        viewport.height - cardHeight - cardMargin,
      ),
      placement,
    };
  }

  const cardLeft = clamp(
    rect.left + rect.width / 2 - cardWidth / 2,
    cardMargin,
    viewport.width - cardWidth - cardMargin,
  );
  const cardTop = placement === "below"
    ? rect.bottom + cardGap
    : rect.top - cardGap - cardHeight;
  return {
    arrow: placement === "below" ? "↑" : "↓",
    arrowLeft: clamp(rect.left + rect.width / 2 - 18, 8, viewport.width - 44),
    arrowTop: placement === "below" ? rect.bottom + 4 : rect.top - 40,
    cardLeft,
    cardTop: clamp(cardTop, cardMargin, viewport.height - cardHeight - cardMargin),
    placement,
  };
};

const clamp = (value: number, minimum: number, maximum: number): number =>
  Math.min(Math.max(value, minimum), maximum);
