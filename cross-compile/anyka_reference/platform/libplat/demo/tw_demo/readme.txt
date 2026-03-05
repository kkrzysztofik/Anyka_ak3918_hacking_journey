anyka tone wave demo
This example program introduces:
1. Intelligent voice control recognition, which parses and places the result into an output buffer. If used for sonic matching, you can call network setting interfaces after obtaining the parsed SSID and password.
2. Intelligent voice control WAV generation, which forms a voice control string from the SSID and password. The generated data can be played on a phone or PC. The generation interface and corresponding library can be integrated directly into a mobile app.

3. Call examples:
3.1. Generate intelligent voice control WAV audio:
	./ak_tw_demo ssid password save_wav_file
	This example program uses the SSID and password of a wireless router to generate a corresponding wave format file.

3.2. Intelligent voice control recognition:
	./ak_tw_demo
	The timeout set in the example program is 60 seconds.
