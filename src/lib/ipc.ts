import { invoke } from "@tauri-apps/api/core";
import type { AppError, Settings } from "@/types";
import { isAppError } from "@/types";

/**
 * Registry of every Rust command exposed via `generate_handler!` in
 * `src-tauri/src/lib.rs`. Keys are command names; `args` uses the
 * camelCase names Tauri expects, `result` is the deserialized return type.
 */
interface Commands {
  get_settings: { args: Record<string, never>; result: Settings };
  update_settings: { args: { settings: Settings }; result: Settings };
}

type CommandName = keyof Commands;

type ArgsParam<K extends CommandName> =
  Commands[K]["args"] extends Record<string, never> ? [] : [args: Commands[K]["args"]];

/** Typed wrapper around `invoke()`. Rejects with a normalized `AppError`. */
export async function call<K extends CommandName>(
  command: K,
  ...args: ArgsParam<K>
): Promise<Commands[K]["result"]> {
  try {
    return await invoke<Commands[K]["result"]>(command, args[0]);
  } catch (err: unknown) {
    throw toAppError(err);
  }
}

function toAppError(err: unknown): AppError {
  if (isAppError(err)) return err;
  return { kind: "tauri", message: typeof err === "string" ? err : String(err) };
}
