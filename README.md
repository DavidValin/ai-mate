## ai-mate

ai mate is a terminal based audio conversation system between a user and an AI model that runs locally in your machine.

- llm system: ollama
- speech to text (stt): whisper.cpp
- text to speech (tts): OpenTTS

### How it works

`RECORD -> SPEECH TO TEXT -> OLLAMA -> RESPONSE -> TEXT TO SPEECH -> PLAY AUDIO`

```
- `You start the program and start talking.`
- `As soon as there is a silence (based on sound-threshold-peak option), it will transcribe your audio speech into text sent to the ai model.`
- `The AI model will reply with text, converted to audio using text to speech and played.`
- `You can interrupt the ai agent at any moment by start speaking, this will cause the response and audio to stop and you can continue talking.`
- `Pressing space during audio playback.`
```

This is how internally works:

```
┌──────────────────────────────┐        ┌──────────────────────────────┐
│            MAIN              │        │          UI (thread)         │
├──────────────────────────────┤        ├──────────────────────────────┤
│ parse args                   │        │ status line                  │
│ select audio configs         │        │ spinner                      │
│ create channels              │        └──────────────────────────────┘
│ spawn threads                │
└───────────────┬──────────────┘
                │
                │
┌───────────────▼──────────────┐        ┌──────────────────────────────┐
│       RECORD (thread)        │        │      KEYBOARD (thread)       │
├──────────────────────────────┤        ├──────────────────────────────┤
│ mic capture                  │        │ space  -> pause              │
│ voice activity detect        │        │ esc    -> shutdown           │
│ detect speech while playing  │        │ ctrl-c -> shutdown           │
│ interrupt_counter += 1       │        └───────────────┬──────────────┘
│ send stop_play               │                        │
│ send utterance audio         │                        │
└───────────────┬──────────────┘                        ▼
                │ utterance                    ┌──────────────────────────┐
                │                              │     PLAYBACK (thread)    │
┌───────────────▼──────────────┐               ├──────────────────────────┤
│   CONVERSATION (thread)      │               │ audio queue              │
├──────────────────────────────┤               │ output callback          │
│ Whisper: speech -> text      │               │ clear queue on stop      │
│ LLM token stream             │               │ pause -> silence         │
│                              │               └───────────────▲──────────┘
│  ┌────────────────────────┐  │                               │
│  │      PHRASE QUEUE      │  │                               │
│  │     (text buffer)      │  │                               │
│  │ wait for boundary      │  │                               │
│  └───────────┬────────────┘  │                               │
│              │ phrase ready  │                               │ stop_play
│ OpenTTS: text -> speech      │                               │
│ stop on interrupt            │                               │
│ send audio chunks            │                               │
└───────────────┬──────────────┘                               │
                │ audio chunks                                 │
                ▼                                              │
┌──────────────────────────────┐                               │
│        ROUTER (thread)       │                               │
├──────────────────────────────┤                               │
│ receive audio chunks         │                               │
│ channel mapping              │                               │
│ forward to playback          │───────────────────────────────┘
└──────────────────────────────┘

```

### Installation

Install dependencies:

- Docker: `https://docs.docker.com/engine/install` (needed for STT)
- Ollama: `https://ollama.com/download` (needed for ai responses)
- Whisper.cpp: `https://github.com/ggml-org/whisper.cpp`, see 'Quick Start' (needed for TTS)
- Rust: `https://rustup.rs` (needed to compile ai-mate from source)
- Alsa development libraries: called `libasound2-dev` or `alsa-lib-devel` or `alsa-lib`
- Install `pkg-config`

Download models:

- `ollama pull deepseek-r1:latest` (or the model you want to use)
- `https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-q5_0.bin?download=true`

Build and install ai-mate:

```
cargo build --release
cargo install --path .
```

This installs the program called `ai-mate` under `~/.cargo/bin`. Make sure this directory is added to your path, otherwise add it.

### How to use it

Before starting, make sure ollama and OpenTTS are running:

- Terminal 1: `ollama serve`
- Terminal 2: `docker run --rm --platform=linux/amd64 -p 5500:5500 synesthesiam/opentts:all` (it will pull the image the first time). Adjust the platform as needed depending on your hardware. This container contains all the voices for all languages.

To start the conversation follow this instructions:

Below is the default parameters, which you can override, example:

```
ai-mate 
  --language en
  --voice larynx:cmu_fem-glow_tts
  --sound-threshold-peak 0.10 \
  --end-utterance-silence-ms 850 \
  --whisper-model-path "$HOME/.whisper-models/ggml-large-v3-q5_0.bin" \
  --ollama-url "http://localhost:11434/api/generate" \
  --ollama-model "deepseek-r1:latest" \
  --opentts-base-url "http://0.0.0.0:5500/api/tts?voice=coqui-tts%3Aen_ljspeech&lang=en&vocoder=high&denoiserStrength=0.005&&speakerId=&ssml=false&ssmlNumbers=true&ssmlDates=true&ssmlCurrency=true&cache=false"
```

You can just override a specific variable, for example:

```
ai-mate --ollama-model "llama3.2:3b --language es"
```

If you need help:

```
ai-mate --help
```

### Language support

By default everything run in english (speech recognition and audio playback). The next languages are supported:

```
Language ID         DEFAULT VOICE                              LANGUAGE NAME
____________________________________________________________________________

ar                  festival:ara_norm_ziad_hts                 arabic
bn                  flite:cmu_indic_ben_rm                     bengali
ca                  festival:upc_ca_ona_hts                    catalan
cs                  festival:czech_machac                      czech
de                  glow-speak:de_thorsten                     german
el                  glow-speak:el_rapunzelina                  greek
en                  larynx:cmu_fem-glow_tts                    english
es                  larynx:karen_savage-glow_tts               spanish
fi                  glow-speak:fi_harri_tapani_ylilammi        finnish
fr                  larynx:gilles_le_blanc-glow_tts            french
gu                  flite:cmu_indic_guj_ad                     gujarati
hi                  flite:cmu_indic_hin_ab                     hindi
hu                  glow-speak:hu_diana_majlinger              hungarian
it                  larynx:riccardo_fasol-glow_tts             italian
ja                  coqui-tts:ja_kokoro                        japanese
kn                  flite:cmu_indic_kan_plv                    kannada
ko                  glow-speak:ko_kss                          korean
mr                  flite:cmu_indic_mar_aup                    marathi
nl                  glow-speak:nl_rdh                          dutch
pa                  flite:cmu_indic_pan_amp                    punjabi
ru                  glow-speak:ru_nikolaev                     russian
sv                  glow-speak:sv_talesyntese                  swedish
sw                  glow-speak:sw_biblia_takatifu              swahili
ta                  flite:cmu_indic_tam_sdr                    tamil
te                  marytts:cmu-nk-hsmm                        telugu
tr                  marytts:dfki-ot-hsmm                       turkish
zh                  coqui-tts:zh_baker                         mandarin chinese
```

Feel free to contribute using a PR.
Have fun o:)

### Language support