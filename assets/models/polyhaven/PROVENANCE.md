# Poly Haven model provenance

All three source models are CC0 under [Poly Haven’s asset license](https://polyhaven.com/license). They are bundled for the local Windows visual review only. The source files are ordinary glTF with external binary and JPEG dependencies; none declares a glTF compression extension. The app uses Bevy 0.18’s glTF loader and JPEG decoder.

The 1K variant was selected to limit disk and VRAM use. Exact source and bundled SHA-256 values follow.

## [Food Apple 01](https://polyhaven.com/a/food_apple_01)

Author: Oliver Harries. Source variant: 1K glTF.

| Bundled file | Exact source URL | SHA-256 |
| --- | --- | --- |
| `food_apple_01/food_apple_01_1k.gltf` | https://dl.polyhaven.org/file/ph-assets/Models/gltf/1k/food_apple_01/food_apple_01_1k.gltf | `6a7dc1cda1271c8b08e36e2b9fd44371b7149cca6262c71bc36d971cdac4325f` |
| `food_apple_01/textures/food_apple_01_diff_1k.jpg` | https://dl.polyhaven.org/file/ph-assets/Models/jpg/1k/food_apple_01/food_apple_01_diff_1k.jpg | `b7fc17cfb5013958167bbc089ea4d1c193fb1c61ea4f3c974a9894ed9ba82e8a` |
| `food_apple_01/food_apple_01.bin` | https://dl.polyhaven.org/file/ph-assets/Models/gltf/8k/food_apple_01/food_apple_01.bin | `f14ff9647fdd9f5a59ef325f94154e91f1f14cc2563cd3fb572c4fe9ce5756ca` |
| `food_apple_01/textures/food_apple_01_nor_gl_1k.jpg` | https://dl.polyhaven.org/file/ph-assets/Models/jpg/1k/food_apple_01/food_apple_01_nor_gl_1k.jpg | `c81b1e51ce7e789b65bfde2f046dcb5ba5773b5c6be3e572313be0600c51721b` |
| `food_apple_01/textures/food_apple_01_rough_1k.jpg` | https://dl.polyhaven.org/file/ph-assets/Models/jpg/1k/food_apple_01/food_apple_01_rough_1k.jpg | `2e5a503af38df7b570b0e81cc9563125f0913b466e01140d4e24ea5ebdbec587` |

## [Food Avocado 01](https://polyhaven.com/a/food_avocado_01)

Author: Oliver Harries. Source variant: 1K glTF.

| Bundled file | Exact source URL | SHA-256 |
| --- | --- | --- |
| `food_avocado_01/food_avocado_01_1k.gltf` | https://dl.polyhaven.org/file/ph-assets/Models/gltf/1k/food_avocado_01/food_avocado_01_1k.gltf | `cce51afb7dcdc6c19371806993e2c1096788a10accf453fda1198875e59f7846` |
| `food_avocado_01/textures/food_avocado_01_diff_1k.jpg` | https://dl.polyhaven.org/file/ph-assets/Models/jpg/1k/food_avocado_01/food_avocado_01_diff_1k.jpg | `b6953e91378f491d7aadde14f6dd9e83b22d37b160dcdb944c3f7b35454bae44` |
| `food_avocado_01/textures/food_avocado_01_nor_gl_1k.jpg` | https://dl.polyhaven.org/file/ph-assets/Models/jpg/1k/food_avocado_01/food_avocado_01_nor_gl_1k.jpg | `325b29fb90b621687507eb099993597dd160a43666c2556f0bd717d217902231` |
| `food_avocado_01/textures/food_avocado_01_rough_1k.jpg` | https://dl.polyhaven.org/file/ph-assets/Models/jpg/1k/food_avocado_01/food_avocado_01_rough_1k.jpg | `c830a9276d3069fb0a587d8630b0d8771724279e0e105afb67bf63ab923ca5e0` |
| `food_avocado_01/food_avocado_01.bin` | https://dl.polyhaven.org/file/ph-assets/Models/gltf/8k/food_avocado_01/food_avocado_01.bin | `72503b157d307643cc7dc05102d9a8921a091de512c8c42c5a7a1716f99e113f` |

## [Long Life Food](https://polyhaven.com/a/long_life_food)

Author: Mia Pecina Zorko. Source variant: 1K glTF.

The original source texture atlas contains designed packaging names and labels. It is **not bundled**. Four local derivative glTF files retain the CC0 geometry, remove all image/texture references, and assign original flat generic material colors. The app adds original geometric color-block labels at runtime. Only confirmed matching stock creates a scene.

| Bundled file | Source or derivation | SHA-256 |
| --- | --- | --- |
| `long_life_food/long_life_food.bin` | https://dl.polyhaven.org/file/ph-assets/Models/gltf/4k/long_life_food/long_life_food.bin | `efd51e3ea3aa7cb047c33792a484466e105f9ffd88cc1c508bf73dcafabed42e` |
| Source glTF (not bundled) | https://dl.polyhaven.org/file/ph-assets/Models/gltf/1k/long_life_food/long_life_food_1k.gltf | `c96276210b99df049253fdb4541215439eff7e5e1fb960a0f20fe68f6bc12b07` |
| `long_life_food/generic_beans.gltf` | Local derivative of the source glTF above | `6da74a14ae1004150a8339a20ce1772aa844d8fa63f26a70e24edeb987661b72` |
| `long_life_food/generic_milk.gltf` | Local derivative of the source glTF above | `311b655c8a472baa89d37bb88b3d1d21f1bbbec5d4809884f88e58d37953af35` |
| `long_life_food/generic_sardines.gltf` | Local derivative of the source glTF above | `484586b4cca36c5ef7b708b6435c4c3595444e63f2dea8ade112a046c8718af1` |
| `long_life_food/generic_tomatoes.gltf` | Local derivative of the source glTF above | `f7d05abf42ce2644af5f38734bf5f06c9b3fa5a35f0836ececa7bc72d2fd0485` |

No equirectangular HDRI is bundled; the existing Bevy light rig remains in use.
