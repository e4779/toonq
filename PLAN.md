# PLAN.md — toonq

## Статус: ✅ Завершён

**313 строк Rust, 1.8MB бинарь, 11 тестов.**

## Что это

`toonq` — jq для TOON. CLI-утилита для запросов, фильтрации, инспекции и конвертации TOON-файлов.

## Архитектура

```
TOON → serde_toon (Rust) → JSON → jaq -c → JSON → serde_toon → TOON
         нативный           ↑  subprocess  ↑        нативный
         парсинг            └── компактный вывод ──┘  вывод
```

**Почему jaq subprocess, а не библиотека:**
- `jaq-core` v3 — слишком абстрактный API для простого использования
- `oq` v0.1 — не компилируется
- `jaq` binary + `-c` — работает идеально, полная jq-совместимость

## Стек

| Крейт | Назначение |
|-------|-----------|
| `serde_toon_format` 0.1.2 | TOON ↔ serde_json::Value |
| `serde_json` 1 | Промежуточный JSON |
| `jaq` v3.0.0 | jq-движок (subprocess) |
| `clap` 4 | CLI |
| `anyhow` 1 | Ошибки |

## API

```bash
# Инспекция
toonq --head 5 data.toon           # первые N записей
toonq --tail 3 data.toon           # последние N
toonq --count data.toon            # количество записей
toonq --schema data.toon           # поля и типы
toonq --stats data.toon            # статистика токенов

# Запросы (полный jq-синтаксис)
toonq -f '.[] | select(.close > 100)' data.toon
toonq -f 'sort_by(-.close) | .[0:3]' data.toon

# Формат вывода
toonq --to json data.toon          # TOON → JSON
toonq -f '.[]' --to raw data.toon  # Сырой jaq-вывод (без TOON-обёртки)

# Формат ввода
toonq --from json data.json        # JSON → TOON
toonq data.toon                    # авто-детект по расширению

# Пайплайн
toonq -f 'filter' data.toon | toonq --head 3
toonq --to json data.toon | toonq --from json --count
```
