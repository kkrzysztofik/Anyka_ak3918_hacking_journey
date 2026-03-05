Extended test example for AEC (Acoustic Echo Cancellation) function. The demo tests the opening order of AI and AO and whether the AEC function is enabled. For testing, place a file named talk.pcm in the /tmp/ directory. The talk.pcm should have a sample rate of 8k and 16-bit sample bits.

Demo input parameters: ak_aec_ex_demo value
Example input: ak_aec_ex_demo 1

The range of value is 1~6, with the following meanings:
1: Open AI module without AEC, do not open AO module. Speakers will not play audio. Results saved to /tmp/ as a PCM file (8k sample rate, 1 minute capture, volume 3). Only captured mic sound will be heard.
2: Open AI module with AEC, do not open AO module. Speakers will not play audio. Results saved to /tmp/ as a PCM file (8k sample rate, 1 minute capture, volume 3). Only captured mic sound will be heard.
3: Open AI module first, then AO module, without AEC. talk.pcm will be played. Results saved to /tmp/ as a PCM file (8k sample rate, 1 minute capture, volume 3). Both captured mic sound and speaker audio will be heard.
4: Open AI module first, then AO module, with AEC enabled. talk.pcm will be played. Results saved to /tmp/ as a PCM file (8k sample rate, 1 minute capture, volume 3). Captured mic sound will be heard, but speaker audio will be cancelled by AEC.
5: Open AO module first, then AI module, without AEC. talk.pcm will be played. Results saved to /tmp/ as a PCM file (8k sample rate, 1 minute capture, volume 3). Both captured mic sound and speaker audio will be heard.
6: Open AO module first, then AI module, with AEC enabled. talk.pcm will be played. Results saved to /tmp/ as a PCM file (8k sample rate, 1 minute capture, volume 3). Captured mic sound will be heard, but speaker audio will be cancelled by AEC.

Note:
1. The AEC function switch is in the AI module; the AO module does not have an AEC switch.
2. Since talk.pcm is placed in /tmp/, do not use files that are too large to avoid memory issues.
3. anyka_ipc must be killed first (service.sh stop) before running this demo.
4. Use voice broadcast type audio for talk.pcm; avoid music for better test results.
