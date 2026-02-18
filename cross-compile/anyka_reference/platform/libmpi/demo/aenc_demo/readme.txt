aenc module call example, providing the call method for the aenc module interface.
Demo input parameters: ak_aenc_demo [sample rate] [audio type] [save path] [save time (unit: seconds)] [volume] [capture source (mic/linein)]
Example input: ak_aenc_demo 8000 mp3 /mnt/ 10 7 mic

Testers calling the aenc demo can encode captured raw audio data, which is then saved to the specified path. By checking the saved audio files, testers can determine if the audio encoding is working properly.

Note:
1. Note that the save path must end with "/".
2. Supported audio encoding formats include mp3/amr/aac/g711a/g711u/pcm.
3. For saved audio files, AAC does not contain header information, so it cannot be played with standard players, but can be played with the adec demo;
4. For saved audio files, G.711a and G.711u do not contain header information and can be played with Cool Edit;
5. When testing this demo, anyka_ipc must be killed first. The command to kill anyka_ipc is "service.sh stop";
6. For AAC, MP3, G.711a, and G.711u formats, it supports capturing files with a sample rate no greater than 48000, sample bits of 16, and mono channel;
7. Captured audio is all mono audio;
