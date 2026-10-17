# Задачи v1

- [x] T1. Cargo + scene: отдельные mesh/instances, габариты; проверить core unit tests.
- [x] T2. Import: ZIP/OPC/XML, units/transforms/components; проверить синтетические integration fixtures.
- [x] T3. Compatibility: Production parts, Bambu роли/plates, warnings; проверить состав и координаты сцен.
- [x] T4. Camera/settings: views/projection/pan/zoom, атомарные настройки; проверить матрицы и сохранение.
- [x] T5. Loader: один worker, отмена, последняя загрузка; проверить гонки и восстановление после ошибки.
- [x] T6. UI/GPU: модель в окне, диалог/drop/CLI, фон и статистика; проверить native build и окно.
- [x] T7. Distribution: macOS bundle, Windows/Linux CI и packaging; проверить доступный локальный артефакт.
- [x] T8. Review/docs: fmt, clippy, tests, release build; обновить README/SPEC/AGENTS и отчёт проверки.

Все восемь этапов реализации выполнены. Это не закрывает полную платформенную приёмку AC-01…AC-12; её фактическая матрица и ограничения — в [отчёте](../docs/VALIDATION.md).

Проверки на недоступных ОС и решение владельца о лицензии фиксируются отдельно в отчёте готовности и не заменяются предположениями.
