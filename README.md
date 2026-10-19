<h1 align="center">Lexicon</h1>

<p align="center">
  <b>Текстовый процессор с проверкой орфографии на русском и английском — для Android, Windows, macOS, Linux и браузера.</b><br>
  <i>A word processor with Russian and English spelling, for Android, Windows, macOS, Linux and the web.</i>
</p>

Lexicon основан на [WordCraft](https://github.com/storytold/wordcraft) — открытом текстовом
процессоре на Rust с интерфейсом в стиле Word (лента, стили, таблицы, рецензирование, `.docx`).
Отличия Lexicon:

- **Русская орфография из коробки.** Словарь `ru_RU` (LibreOffice, автор — А. И. Лебедев,
  лицензия BSD) встроен в программу. В WordCraft русский проверялся только при установленном в
  системе словаре Hunspell, а на Android и в браузере не проверялся вовсе.
- **Русский и английский одновременно.** Каждое слово проверяется по словарю своего алфавита:
  кириллица — по русскому, латиница — по английскому, какой бы язык ни был выбран для текста.
  В одном абзаце можно писать «Я купил новый iPhone and a bottle of milk» — ошибки подчеркнутся
  в обоих языках, подсказки предложит нужный словарь.
- **Android-приложение** (APK) с экранной клавиатурой, включая набор с подсказками
  (Gboard, SwiftKey и т. п.).

## Установка на Android

1. Скачайте APK на телефон по прямой ссылке: **https://github.com/Arthuralend/Lexicon/releases/download/latest/Lexicon.apk** — там всегда последняя сборка.
2. Откройте скачанный `Lexicon.apk`. Разрешите установку из
   этого источника, когда телефон спросит.
3. При первом запуске Lexicon попросит доступ к файлам — тогда документы можно открывать и
   сохранять в папках «Документы» и «Загрузки». Без доступа документы хранятся в папке
   приложения (`Android/data/io.github.arthuralend.lexicon/files`).

Требования: Android 8.0 или новее, 64-битный процессор ARM (arm64-v8a) — это почти все
современные телефоны.

Чтобы выпустить новую версию с APK на странице Releases, поставьте тег: `git tag v0.8.0 && git push origin v0.8.0`.

## Сборка

```sh
# Настольная версия
cargo run --release -p wordcraft -- --sample

# Android (нужны Android SDK и NDK, Java 17, Gradle 8.11, cargo-ndk)
rustup target add aarch64-linux-android
cargo ndk -t arm64-v8a --platform 26 -o android/app/src/main/jniLibs build --release -p lexicon-android
cd android && gradle assembleRelease      # → android/app/build/outputs/apk/release/app-release.apk

# Браузер
cd apps/wordcraft-web && trunk serve
```

Устройство Android-версии:

| Часть | Где |
|---|---|
| Rust-оболочка (запуск, папки, настройки) | [`apps/lexicon-android/src/lib.rs`](apps/lexicon-android/src/lib.rs) |
| Экранная клавиатура (IME → события egui) | [`apps/lexicon-android/src/keyboard.rs`](apps/lexicon-android/src/keyboard.rs), [`LexiconActivity.java`](android/app/src/main/java/io/github/arthuralend/lexicon/LexiconActivity.java) |
| Gradle-проект, манифест, иконка | [`android/`](android/) |
| Сборка APK в CI | [`.github/workflows/android.yml`](.github/workflows/android.yml) |
| Русский словарь и проверка двух языков | [`crates/proof/src/ru/mod.rs`](crates/proof/src/ru/mod.rs), `by_script` в [`crates/proof/src/lib.rs`](crates/proof/src/lib.rs) |

APK подписывается ключом `android/app/lexicon.keystore`, который лежит в репозитории, чтобы
каждая сборка устанавливалась поверх предыдущей. Ключ публичный; для публикации в магазине
приложений подпишите своим ключом через переменные `LEXICON_KEYSTORE`, `LEXICON_KEYSTORE_PASSWORD`,
`LEXICON_KEY_ALIAS`, `LEXICON_KEY_PASSWORD`.

Внутренние крейты сохранили имена `wordcraft-*`, чтобы изменения из WordCraft было проще переносить.
Подробное описание возможностей редактора — в [README WordCraft](docs/WORDCRAFT-README.md).

## Лицензия

Lexicon, как и WordCraft, распространяется по лицензии [MIT](LICENSE-MIT) или
[Apache-2.0](LICENSE-APACHE) на ваш выбор. Copyright (c) 2026 ArtCraft Team и участники проектов
WordCraft и Lexicon; обязательные уведомления — в [NOTICE](NOTICE).

Встроенные шрифты, словари и прочие ресурсы сохраняют свои лицензии; каждый указан в
[ATTRIBUTION.md](ATTRIBUTION.md). Русский словарь: Copyright (c) 1997–2008 Alexander I. Lebedev,
лицензия в [`assets/spelling/README_ru_RU.txt`](assets/spelling/README_ru_RU.txt). Английский
словарь и переносы — общественное достояние (Moby Hyphenator II, Grady Ward).

Lexicon — независимый проект, не связанный с ArtCraft, Microsoft или Adobe. Логотипы ArtCraft
из репозитория WordCraft удалены, как того требует их лицензия.
