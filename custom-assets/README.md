# Your own boards

Put deck graphics in `decks/` and run `install-gta.ps1` again. Each image
(PNG, JPG, BMP, TGA, DDS or WebP; subfolders too) becomes a deck in the board
store, beside Skate 3's own; nothing is replaced.

| Image | Becomes |
|---|---|
| A square image of 256 px or more | a whole Skate deck texture, as Skate 3 texture mods have it (grip side in the top half, graphic in the bottom half, along the board) |
| Any other image | a graphic, laid onto the bottom of a plain deck (turned along the board when it stands upright, cropped to fill) |

The store sorts it under the brand named by its folder or file name
(`decks/Baker/my deck.png` is a Baker deck), else under "Custom". The deck's
name comes from the file name (`my_cool-deck.png` is "My Cool Deck").

Only use images you made yourself or may use.
