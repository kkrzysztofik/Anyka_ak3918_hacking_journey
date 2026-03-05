Call example for the AO module, providing call methods for AO module interfaces.
Demo input parameters: ak_ao_demo [sample rate] [full path of file to play] [volume]
Example input: ak_ao_demo 8000 /mnt/a.pcm 7

Testers calling the AO demo to play PCM files can listen through speakers or headphones to determine if the played audio is normal.

Note:
1. Note that the save path must end with "/";
2. anyka_ipc must be killed before testing this demo;
3. If the PCM file a.pcm to be played is placed in /tmp/, do not use files that are too large to avoid memory issues.
