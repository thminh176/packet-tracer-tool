# Adding and updating languages

The translation catalog is data, independent of egui widgets. `locales/en.json` is the base contract; stable semantic keys such as `menu.open_file` and `nav.overview` are used by the UI. `locales/vi.json` supplies Vietnamese. Both are embedded so a standalone executable always has a fallback.

## Add a language without rebuilding

1. Copy `locales/en.json` into the `locales` folder beside the EXE, for example `locales/fr.json`.
2. Set `locale` to a code such as `fr` or `pt-BR`, and `name` to its native display name.
3. Translate the message values. Keep keys and `{placeholder}` names unchanged.
4. Restart the application and select the new language. The choice is saved with preferences.

New packs are discovered automatically; no Rust registration or enum changes are needed. Missing or empty translations fall back to the embedded English catalog. Invalid JSON, unknown keys, mismatched placeholders, invalid locale names, or packs larger than 1 MiB are rejected and reported through localization diagnostics. Invalid packs do not prevent startup.

## Update an existing translation

Place a partial pack with `locale: "vi"` beside the EXE to override Vietnamese strings. Only supply the keys you want to change. Embedded strings fill gaps. English external overrides affect displayed English; the embedded base remains the fallback contract for missing translations.

## Add a new message in development

Add the same semantic key to both `en.json` and `vi.json`, use `Localizer::text(locale, key)` from the UI, and run `cargo test`. For dynamic values, use `Localizer::format(locale, key, &[("device", name)])`; preserve placeholder names across languages. Raw device names, interfaces, XML, expected values, and Cisco CLI stay unchanged. `Localizer::message` supports migrating existing diagnostic explanations by matching their English base value; new features should prefer stable keys.

Language packs affect displayed text only. They cannot add validators, generate executable commands, execute scripts, or change grading results.

## Tiếng Việt

Sao chép `en.json`, đổi `locale` và `name`, rồi dịch phần `messages`. Giữ nguyên tên khóa và các biến trong dấu `{}`. Đặt tệp trong thư mục `locales` cạnh EXE và mở lại ứng dụng. Khóa thiếu sẽ dùng tiếng Anh; không cần sửa mã Rust để thêm ngôn ngữ.
