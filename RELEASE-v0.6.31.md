# Proton Medic v0.6.31

- Fixed Windows game name validation to count Unicode characters instead of UTF-8 bytes. Names containing Chinese, Japanese, Korean, and other non-ASCII characters now use the same 120-character limit as ASCII names.
- Kept Restore buttons within the window when a removed game has a long, unbroken name.
- Existing profiles, game files, prefixes, and shortcuts are preserved.
