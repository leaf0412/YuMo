import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerDictionaryHandlers(): void {
  // --- Vocabulary ---
  ipcMain.handle("get-vocabulary", () => {
    return JSON.parse(getAddon().getVocabulary());
  });

  ipcMain.handle("add-vocabulary", (_e, args?: { word?: string }) => {
    if (args?.word) return getAddon().addVocabulary(args.word);
  });

  ipcMain.handle("delete-vocabulary", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deleteVocabulary(args.id);
  });

  // --- Replacements ---
  ipcMain.handle("get-replacements", () => {
    return JSON.parse(getAddon().getReplacements());
  });

  ipcMain.handle("set-replacement", (_e, args?: { original?: string; replacement?: string }) => {
    if (args?.original && args?.replacement) {
      return getAddon().setReplacement(args.original, args.replacement);
    }
  });

  ipcMain.handle("delete-replacement", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deleteReplacement(args.id);
  });

  // --- Prompts ---
  ipcMain.handle("list-prompts", () => {
    return JSON.parse(getAddon().listPrompts());
  });

  ipcMain.handle(
    "add-prompt",
    (_e, args?: { name?: string; systemMsg?: string; userMsg?: string }) => {
      if (args?.name && args?.systemMsg !== undefined && args?.userMsg !== undefined) {
        return getAddon().addPrompt(args.name, args.systemMsg, args.userMsg);
      }
    },
  );

  ipcMain.handle(
    "update-prompt",
    (_e, args?: { id?: string; name?: string; systemMsg?: string; userMsg?: string }) => {
      if (args?.id && args?.name !== undefined && args?.systemMsg !== undefined && args?.userMsg !== undefined) {
        getAddon().updatePrompt(args.id, args.name, args.systemMsg, args.userMsg);
      }
    },
  );

  ipcMain.handle("delete-prompt", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deletePrompt(args.id);
  });

  ipcMain.handle("select-prompt", (_e, args?: { id?: string }) => {
    if (args?.id) {
      getAddon().updateSetting("selected_prompt_id", JSON.stringify(args.id));
    }
  });

  // --- CSV Import/Export ---
  ipcMain.handle(
    "import-dictionary-csv",
    (_e, args?: { path?: string; dictType?: string }) => {
      if (args?.path && args?.dictType) {
        getAddon().importDictionaryCsv(args.path, args.dictType);
      }
    },
  );

  ipcMain.handle(
    "export-dictionary-csv",
    (_e, args?: { path?: string; dictType?: string }) => {
      if (args?.path && args?.dictType) {
        getAddon().exportDictionaryCsv(args.path, args.dictType);
      }
    },
  );
}
