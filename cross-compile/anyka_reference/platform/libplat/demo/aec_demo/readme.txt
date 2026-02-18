Test example for AEC (Acoustic Echo Cancellation) function. The demo implements echo cancellation by calling AI and AO module interfaces.
Demo input parameters: ak_aec_demo [sample rate] [save path] [save time] [volume] [full path of PCM audio to play]
Example input: ak_aec_demo 8000 /mnt/ 120 7 /mnt/a.pcm

After the test starts, audio will be played from the specified PCM file. While playing, the tester speaks into the mic. The captured data will be saved to the specified path. If the echo cancellation is effective, the saved file will contain the tester's voice but not the played audio.

Note:
1. Note that the save path must end with "/";
2. If the save path is set to /tmp/, do not record for too long to avoid memory issues. PCM files are best saved on the T-card;
3. anyka_ipc must be killed (service.sh stop) before running this demo;
4. If the PCM file a.pcm to be played is in /tmp/, do not use files that are too large;
5. Use voice broadcast type audio for a.pcm; avoid music for better test results;
6. Use voice instead of music during mic capture as well.
