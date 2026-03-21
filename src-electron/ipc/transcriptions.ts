import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerTranscriptionsHandlers(): void {
  ipcMain.handle(
    "get-transcriptions",
    async (_e, args?: { cursor?: string; query?: string; limit?: number }) => {
      const json = await getAddon().getTranscriptions(
        args?.cursor ?? null,
        args?.query ?? null,
        args?.limit ?? null,
      );
      return JSON.parse(json);
    },
  );

  ipcMain.handle("delete-transcription", async (_e, args?: { id?: string }) => {
    if (args?.id) await getAddon().deleteTranscription(args.id);
  });

  ipcMain.handle("delete-all-transcriptions", async () => {
    await getAddon().deleteAllTranscriptions();
  });

  ipcMain.handle(
    "get-recording",
    async (_e, args?: { recordingPath?: string }) => {
      if (args?.recordingPath) {
        return await getAddon().getRecording(args.recordingPath);
      }
      return null;
    },
  );

  // --- Statistics ---
  ipcMain.handle("get-statistics", async (_e, args?: { days?: number }) => {
    return JSON.parse(await getAddon().getStatistics(args?.days ?? null));
  });
}
