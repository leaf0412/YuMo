import { registerSettingsHandlers } from "./settings";
import { registerAudioHandlers } from "./audio";
import { registerTranscriptionsHandlers } from "./transcriptions";
import { registerModelsHandlers } from "./models";
import { registerDaemonHandlers } from "./daemon";
import { registerDictionaryHandlers } from "./dictionary";
import { registerKeychainHandlers } from "./keychain";
import { registerSystemHandlers } from "./system";
import { registerSpritesHandlers } from "./sprites";
import { registerWindowHandlers } from "./windows";

export function registerAllHandlers(): void {
  registerSettingsHandlers();
  registerAudioHandlers();
  registerTranscriptionsHandlers();
  registerModelsHandlers();
  registerDaemonHandlers();
  registerDictionaryHandlers();
  registerKeychainHandlers();
  registerSystemHandlers();
  registerSpritesHandlers();
  registerWindowHandlers();
}
