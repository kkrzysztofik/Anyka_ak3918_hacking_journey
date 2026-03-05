adec module call example, providing the call method for the adec module interface.
Demo input parameters: ak_adec_demo [sample rate] [channel number] [audio type] [full path of audio file to play]
Example input: ak_adec_demo 8000 1 mp3 /mnt/20161123-153020.mp3

Supported audio encoding formats include mp3/amr/aac/g711a/g711u/pcm.
Testers calling the adec demo to play audio files can listen through speakers or headphones to determine if the played audio is normal.

Note:
1. If the audio file to be played is placed in the /tmp/ directory, do not use files that are too large, otherwise it may fail to run due to insufficient memory;
2. To ensure testing quality, please use audio files like voice broadcasts; try not to use music;
3. Playing AMR format only supports files with a sample rate of 8000, sample bits of 16, and mono channel;
4. For AAC, MP3, G.711a, and G.711u formats, it supports playing files with a sample rate no greater than 48000, sample bits of 16, and mono channel;
5. When testing this demo, anyka_ipc must be killed first. The command to kill anyka_ipc is "service.sh stop";
