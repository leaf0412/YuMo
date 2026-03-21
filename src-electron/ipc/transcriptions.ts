import { ipcMain } from "electron";
import { getAddon } from "../addon";

export function registerTranscriptionsHandlers(): void {
  ipcMain.handle(
    "get-transcriptions",
    (_e, args?: { cursor?: string; query?: string; limit?: number }) => {
      const json = getAddon().getTranscriptions(
        args?.cursor ?? null,
        args?.query ?? null,
        args?.limit ?? null,
      );
      return JSON.parse(json);
    },
  );

  ipcMain.handle("delete-transcription", (_e, args?: { id?: string }) => {
    if (args?.id) getAddon().deleteTranscription(args.id);
  });

  ipcMain.handle("delete-all-transcriptions", () => {
    getAddon().deleteAllTranscriptions();
  });

  ipcMain.handle(
    "get-recording",
    (_e, args?: { recordingPath?: string }) => {
      if (args?.recordingPath) {
        return getAddon().getRecording(args.recordingPath);
      }
      return null;
    },
  );

  // --- Statistics ---
  ipcMain.handle("get-statistics", (_e, args?: { days?: number }) => {
    return JSON.parse(getAddon().getStatistics(args?.days ?? null));
  });
}
