# Proton Medic v0.6.26

- My Games pauses Windows library polling while the application window is hidden, reducing background filesystem work.
- Returning to the window immediately refreshes the library. Previously loaded games stay visible, and responses from before hiding are ignored.
