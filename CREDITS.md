# Credits

Comilito is made by Ricardo Rodrigues — [colorcrow.me](https://colorcrow.me/) ·
[GitHub](https://github.com/RicardoEPRodrigues). The same information is shown in the app's
About window (`src/components/About.svelte`); keep the two in sync.

## Made with AI

Comilito was created with the help of AI: most of its code and documentation were written by
[Claude](https://www.anthropic.com/claude) (Anthropic), through Claude Code, with Ricardo
Rodrigues directing the design, making the decisions and testing the app.

## Sample audio

Bundled in `src-tauri/resources/sample/` and copied into a new project when the user starts
from the sample.

| File | Work | Author / source | Licence |
|------|------|-----------------|---------|
| `music/chopin-nocturne-op9-no2.mp3` | Nocturne in E-flat major, Op. 9 No. 2 (Chopin) | Recorded by the [Musopen](https://musopen.org/) Kickstarter Project “Set Chopin Free”, via [Classicals.de](https://www.classicals.de/) | [Public Domain Mark 1.0](https://creativecommons.org/publicdomain/mark/1.0/) |
| `ambience/heavy-rain.mp3` | [Heavy Rain Wind](https://soundbible.com/2217-Heavy-Rain-Wind.html) | Daniel Simion, SoundBible | [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/) |
| `ambience/civil-war-drummer.mp3` | [Civil War Drummer](https://soundbible.com/1512-Civil-War-Drummer-.html) | DrumM8, SoundBible | [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/) |
| `effects/tolling-bell.mp3` | [Tolling Bell](https://soundbible.com/2206-Tolling-Bell.html) | Daniel Simion, SoundBible | [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/) |

All files were renamed. The Chopin recording, *Heavy Rain Wind* and *Tolling Bell* were
re-encoded to a lower MP3 bitrate (112 / 96 / 96 kbps) to keep the app small; nothing else was changed.

## Images

- The app icon is, for now, the default [Tauri](https://tauri.app) icon
  (© Tauri Programme within The Commons Conservancy, MIT / Apache-2.0).
- Interface symbols are Unicode emoji, drawn by the system's fonts.

## Inspiration

- [Ascapes Mixer](https://github.com/ilyaly/ascapes-mixer) by ilyaly (MIT). Comilito is an
  independent implementation, not a fork.

## Built with

[Tauri](https://tauri.app), [kira](https://github.com/tesselode/kira),
[Symphonia](https://github.com/pdeljanov/Symphonia), [Svelte](https://svelte.dev), and the other
open-source libraries listed in `src-tauri/Cargo.toml` and `package.json`, each under its own licence.
