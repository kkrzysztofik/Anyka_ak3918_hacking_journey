Call example for the AI module, providing call methods for AI module interfaces.
Demo input parameters: ak_ai_demo [sample rate] [save path] [save time] [volume] [mic/linein]
Example input: ak_ai_demo 8000 /mnt/ 120 7 mic

Testers calling the AI demo can capture PCM audio and save it to a specified path. By checking the captured audio file, testers can determine if the AI module is working properly and if the captured audio is normal.

Note:
1. Note that the save path must end with "/";
2. If the save path is set to the /tmp/ directory, please note that the recording time should not be too long, otherwise it may fail to run due to insufficient memory. PCM files are best saved on the T-card;
3. When testing this demo, anyka_ipc must be killed first. The command to kill anyka_ipc is "service.sh stop";
