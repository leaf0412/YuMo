import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerDictionaryHandlers(): void {
  // --- Vocabulary ---
  ipcMain.handle("get-vocabulary", async () => {
    return JSON.parse(await getAddon().getVocabulary());
  });

  ipcMain.handle("add-vocabulary", async (_e, args?: { word?: string }) => {
    if (args?.word) return await getAddon().addVocabulary(args.word);
  });

  ipcMain.handle("delete-vocabulary", async (_e, args?: { id?: string }) => {
    if (args?.id) await getAddon().deleteVocabulary(args.id);
  });

  // --- Replacements ---
  ipcMain.handle("get-replacements", async () => {
    return JSON.parse(await getAddon().getReplacements());
  });

  ipcMain.handle("set-replacement", async (_e, args?: { original?: string; replacement?: string }) => {
    if (args?.original && args?.replacement) {
      return await getAddon().setReplacement(args.original, args.replacement);
    }
  });

  ipcMain.handle("delete-replacement", async (_e, args?: { id?: string }) => {
    if (args?.id) await getAddon().deleteReplacement(args.id);
  });

  // --- Prompts ---
  ipcMain.handle("list-prompts", async () => {
    return JSON.parse(await getAddon().listPrompts());
  });

  ipcMain.handle(
    "add-prompt",
    async (_e, args?: { name?: string; systemMsg?: string; userMsg?: string }) => {
      if (args?.name && args?.systemMsg !== undefined && args?.userMsg !== undefined) {
        return await getAddon().addPrompt(args.name, args.systemMsg, args.userMsg);
      }
    },
  );

  ipcMain.handle(
    "update-prompt",
    async (_e, args?: { id?: string; name?: string; systemMsg?: string; userMsg?: string }) => {
      if (args?.id && args?.name !== undefined && args?.systemMsg !== undefined && args?.userMsg !== undefined) {
        await getAddon().updatePrompt(args.id, args.name, args.systemMsg, args.userMsg);
      }
    },
  );

  ipcMain.handle("delete-prompt", async (_e, args?: { id?: string }) => {
    if (args?.id) await getAddon().deletePrompt(args.id);
  });

  ipcMain.handle("select-prompt", async (_e, args?: { id?: string }) => {
    if (args?.id) {
      await getAddon().updateSetting("selected_prompt_id", JSON.stringify(args.id));
    }
  });

  // --- CSV Import/Export ---
  ipcMain.handle(
    "import-dictionary-csv",
    async (_e, args?: { path?: string; dictType?: string }) => {
      if (args?.path && args?.dictType) {
        await getAddon().importDictionaryCsv(args.path, args.dictType);
      }
    },
  );

  ipcMain.handle(
    "export-dictionary-csv",
    async (_e, args?: { path?: string; dictType?: string }) => {
      if (args?.path && args?.dictType) {
        await getAddon().exportDictionaryCsv(args.path, args.dictType);
      }
    },
  );
}
