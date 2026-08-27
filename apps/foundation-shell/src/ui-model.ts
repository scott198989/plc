import type { FoundationHealthSuccess } from "@govs/foundation-contract";

export type FoundationViewState =
  | Readonly<{ phase: "error"; message: string }>
  | Readonly<{ phase: "initial" }>
  | Readonly<{ phase: "loading" }>
  | Readonly<{ phase: "success"; result: FoundationHealthSuccess }>;

export type FoundationViewAction =
  | Readonly<{ type: "failed"; message: string }>
  | Readonly<{ type: "started" }>
  | Readonly<{ type: "succeeded"; result: FoundationHealthSuccess }>;

export const initialFoundationViewState: FoundationViewState = {
  phase: "initial",
};

export const reduceFoundationViewState = (
  state: FoundationViewState,
  action: FoundationViewAction,
): FoundationViewState => {
  switch (action.type) {
    case "started":
      return state.phase === "loading" ? state : { phase: "loading" };
    case "succeeded":
      return { phase: "success", result: action.result };
    case "failed":
      return { phase: "error", message: action.message };
  }
};
