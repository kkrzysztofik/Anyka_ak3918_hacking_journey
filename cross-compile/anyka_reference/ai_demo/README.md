# Raw PCM Audio
I was able to compile a [working audio input demo](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/SD_card_contents/anyka_hack/audio_in_raw_demo) from the [chinese repo here](https://github.com/helloworld-spec/qiwen/tree/main/anycloud39ev300). It is very basic, there are no settings and it just records audio at 8k samples/s for 10s to a raw pcm file. The goal is to get the aenc_demo working and have proper encoded mp3 audio instead of raw pcm. At least the mic is usable.

# Compiling

Make sure you have the [cross-compiler installed](https://gitea.raspiweb.com/Gerge/Anyka_ak3918_hacking_journey/src/branch/main/cross-compile).

Run: `./build.sh` (you may need to set execute permission first)
