import { useExecuteQuery } from "./useExecuteQuery";

/** Alias for useExecuteQuery — retained for backwards compatibility with Editor. */
export function useRunQuery() {
  return useExecuteQuery();
}
