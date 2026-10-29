# Proton Medic v0.6.17

- Added optional resource monitoring to Windows game cards during game or installer runs, including Proton Medic desktop shortcut launches.
- Shows CPU usage, estimated resident RAM, peak RAM, and the number of observed runtime and child processes.
- Added a bounded memory trend containing the latest 60 samples, refreshed about every two seconds.
- Monitoring is off by default and follows one selected game. It pauses when the window is hidden and stops when the run ends; requests expire if the app stops responding.
- Missing or stale readings are marked unavailable. Partial process readings are labeled, and reused process IDs do not contribute old CPU counters.

Choose **Windows Games → Monitor resources** on a running game. CPU 100% equals one core. RAM totals can count shared pages more than once, so a rise alone does not establish a memory leak. Monitoring does not stop processes or change game files. Signed DEB and AppImage updates remain available.
