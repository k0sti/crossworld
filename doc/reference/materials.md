# Material Types for Cube Voxels

This document defines the material types used for cube voxels in the game world.

## Material Table

| Index | ID                   | Color     | Description                                    |
| ----- | -------------------- | --------- | ---------------------------------------------- |
| 0     | empty                | #00000000 | Empty / undefined space                        |
| 1     | set_empty            | #00000000 | When merging cubes this sets target cube empty |
| 2     | glass                | #80FFFFFF | Transparent glass                              |
| 3     | ice                  | #80D0FFFF | Ice block                                      |
| 4     | water_surface        | #80007FFF | Water surface                                  |
| 5     | slime                | #8000FF00 | Slime block                                    |
| 6     | honey                | #80FFA500 | Honey block                                    |
| 7     | crystal              | #80FF00FF | Crystal material                               |
| 8     | force_field          | #8000FFFF | Force field barrier                            |
| 9     | portal               | #80AA00FF | Portal material                                |
| 10    | mist                 | #40CCCCCC | Mist/fog                                       |
| 11    | stained_glass_red    | #80FF0000 | Red stained glass                              |
| 12    | stained_glass_green  | #8000FF00 | Green stained glass                            |
| 13    | stained_glass_blue   | #800000FF | Blue stained glass                             |
| 14    | stained_glass_yellow | #80FFFF00 | Yellow stained glass                           |
| 15    | transparent_15       | #80808080 | Reserved transparent slot                      |
| 16    | hard_ground          | #FF664433 | Hard ground/bedrock                            |
| 17    | water                | #8000509F | Water block                                    |
| 18    | dirt                 | #FF8B4513 | Dirt                                           |
| 19    | grass                | #FF3A7D3A | Grass                                          |
| 20    | stone                | #FF808080 | Stone                                          |
| 21    | cobblestone          | #FF6E6E6E | Cobblestone                                    |
| 22    | sand                 | #FFEDC9AF | Sand                                           |
| 23    | sandstone            | #FFC9A770 | Sandstone                                      |
| 24    | gravel               | #FF888888 | Gravel                                         |
| 25    | clay                 | #FFA0A0A0 | Clay                                           |
| 26    | snow                 | #FFFFFFFF | Snow                                           |
| 27    | ice_solid            | #FFB0E0FF | Solid ice                                      |
| 28    | obsidian             | #FF1A0F2E | Obsidian                                       |
| 29    | netherrack           | #FF8B0000 | Netherrack                                     |
| 30    | granite              | #FF9C5D3D | Granite                                        |
| 31    | diorite              | #FFBFBFBF | Diorite                                        |
| 32    | andesite             | #FF6D6D6D | Andesite                                       |
| 33    | marble               | #FFE8E8E8 | Marble                                         |
| 34    | limestone            | #FFDAD0C0 | Limestone                                      |
| 35    | basalt               | #FF2B2B2B | Basalt                                         |
| 36    | wood_oak             | #FFA0826D | Oak wood                                       |
| 37    | wood_spruce          | #FF6B5535 | Spruce wood                                    |
| 38    | wood_birch           | #FFD7CB8D | Birch wood                                     |
| 39    | wood_jungle          | #FF8B6F47 | Jungle wood                                    |
| 40    | wood_acacia          | #FFB8683E | Acacia wood                                    |
| 41    | wood_dark_oak        | #FF4A3829 | Dark oak wood                                  |
| 42    | planks_oak           | #FFC4A672 | Oak planks                                     |
| 43    | planks_spruce        | #FF7C5D3E | Spruce planks                                  |
| 44    | planks_birch         | #FFE3D9A8 | Birch planks                                   |
| 45    | leaves               | #FF2D5016 | Leaves                                         |
| 46    | leaves_birch         | #FF5D8F3A | Birch leaves                                   |
| 47    | leaves_spruce        | #FF3D6030 | Spruce leaves                                  |
| 48    | coal                 | #FF1A1A1A | Coal                                           |
| 49    | iron                 | #FFD8D8D8 | Iron                                           |
| 50    | gold                 | #FFFFD700 | Gold                                           |
| 51    | copper               | #FFB87333 | Copper                                         |
| 52    | silver               | #FFC0C0C0 | Silver                                         |
| 53    | bronze               | #FFCD7F32 | Bronze                                         |
| 54    | steel                | #FF9090A0 | Steel                                          |
| 55    | titanium             | #FF878681 | Titanium                                       |
| 56    | brick                | #FF8B3A3A | Brick                                          |
| 57    | concrete             | #FF9E9E9E | Concrete                                       |
| 58    | concrete_white       | #FFEEEEEE | White concrete                                 |
| 59    | concrete_black       | #FF1E1E1E | Black concrete                                 |
| 60    | asphalt              | #FF333333 | Asphalt                                        |
| 61    | rubber               | #FF2B2B2B | Rubber                                         |
| 62    | plastic              | #FFAAAAAA | Plastic                                        |
| 63    | ceramic              | #FFE0D0C0 | Ceramic                                        |
| 64    | skin_light           | #FFFFD5B4 | Light skin tone                                |
| 65    | skin_medium          | #FFDFB08C | Medium skin tone                               |
| 66    | skin_tan             | #FFC98250 | Tan skin tone                                  |
| 67    | skin_brown           | #FF8B5A3C | Brown skin tone                                |
| 68    | skin_dark            | #FF5D3A1A | Dark skin tone                                 |
| 69    | leather_brown        | #FF6F4E37 | Brown leather                                  |
| 70    | leather_black        | #FF2E2620 | Black leather                                  |
| 71    | leather_tan          | #FFBFA088 | Tan leather                                    |
| 72    | fabric_white         | #FFF0F0F0 | White fabric                                   |
| 73    | fabric_red           | #FFDC143C | Red fabric                                     |
| 74    | fabric_blue          | #FF1E90FF | Blue fabric                                    |
| 75    | fabric_green         | #FF228B22 | Green fabric                                   |
| 76    | fabric_yellow        | #FFFFD700 | Yellow fabric                                  |
| 77    | fabric_purple        | #FF8B008B | Purple fabric                                  |
| 78    | fabric_orange        | #FFFF8C00 | Orange fabric                                  |
| 79    | fabric_pink          | #FFFF69B4 | Pink fabric                                    |
| 80    | fabric_black         | #FF1C1C1C | Black fabric                                   |
| 81    | wool_white           | #FFE0E0E0 | White wool                                     |
| 82    | wool_gray            | #FF808080 | Gray wool                                      |
| 83    | wool_red             | #FFB3312C | Red wool                                       |
| 84    | wool_blue            | #FF3C44AA | Blue wool                                      |
| 85    | sponge               | #FFCCCC55 | Sponge                                         |
| 86    | moss                 | #FF597D35 | Moss                                           |
| 87    | mushroom_red         | #FFFF0000 | Red mushroom                                   |
| 88    | mushroom_brown       | #FF9B7653 | Brown mushroom                                 |
| 89    | coral                | #FFFF7F50 | Coral                                          |
| 90    | bamboo               | #FF8FBC8F | Bamboo                                         |
| 91    | cactus               | #FF587D3E | Cactus                                         |
| 92    | vine                 | #FF3E6C25 | Vine                                           |
| 93    | pumpkin              | #FFFF8000 | Pumpkin                                        |
| 94    | melon                | #FF70B341 | Melon                                          |
| 95    | hay                  | #FFD4AF37 | Hay                                            |
| 96    | bone                 | #FFEDE6D6 | Bone                                           |
| 97    | flesh                | #FFFF8080 | Flesh                                          |
| 98    | slime_green          | #FF00FF00 | Green slime material                           |
| 99    | magma                | #FFFF4500 | Magma                                          |
| 100   | lava_rock            | #FF8B0000 | Lava rock                                      |
| 101   | ash                  | #FF605050 | Ash                                            |
| 102   | charcoal             | #FF2F2F2F | Charcoal                                       |
| 103   | sulfur               | #FFFFFF00 | Sulfur                                         |
| 104   | salt                 | #FFF0F0F0 | Salt                                           |
| 105   | sugar                | #FFFFFFFF | Sugar                                          |
| 106   | paper                | #FFFAF0E6 | Paper                                          |
| 107   | cardboard            | #FFAA8866 | Cardboard                                      |
| 108   | wax                  | #FFFFF3D0 | Wax                                            |
| 109   | tar                  | #FF0F0F0F | Tar                                            |
| 110   | oil                  | #FF3C3020 | Oil                                            |
| 111   | paint_red            | #FFFF0000 | Red paint                                      |
| 112   | paint_green          | #FF00FF00 | Green paint                                    |
| 113   | paint_blue           | #FF0000FF | Blue paint                                     |
| 114   | paint_white          | #FFFFFFFF | White paint                                    |
| 115   | paint_black          | #FF000000 | Black paint                                    |
| 116   | glowstone            | #FFFFFFA0 | Glowstone                                      |
| 117   | redstone             | #FFFF0000 | Redstone                                       |
| 118   | emerald              | #FF50C878 | Emerald                                        |
| 119   | diamond              | #FFB9F2FF | Diamond                                        |
| 120   | ruby                 | #FFE0115F | Ruby                                           |
| 121   | sapphire             | #FF0F52BA | Sapphire                                       |
| 122   | amethyst             | #FF9966CC | Amethyst                                       |
| 123   | topaz                | #FFFFC87C | Topaz                                          |
| 124   | pearl                | #FFFFEFD5 | Pearl                                          |
| 125   | quartz               | #FFFFFFFF | Quartz                                         |
| 126   | amber                | #FFFFBF00 | Amber                                          |
| 127   | reserved_127         | #FF888888 | Reserved material slot                         |

## Notes

- Indices 0-15 are reserved for transparent blocks
- Indices 16-127 are named materials with specific properties
- Indices 128-255 are generic colored blocks using 7-bit RGB encoding (automatically generated)
  - Bits: r:2, g:3, b:2
  - Direct mapping: 128 = black, 255 = white
  - RGB values are approximated using the bit distribution
- Color format is #AARRGGBB (Alpha, Red, Green, Blue)
