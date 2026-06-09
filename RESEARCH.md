# RESEARCH.md — toonq

## Идея

CLI-утилита для запросов, фильтрации и инспекции TOON-данных — аналог `jq`, работающий нативно с TOON-форматом.

## Реализация

**215 строк Rust**, бинарь 1.7MB. Зависимости:
- `serde_toon_format` 0.1.2 — TOON ↔ serde_json::Value
- `jaq` v3.0.0 — движок jq (как subprocess, не библиотека)
- `clap` 4 — CLI

## Исследование экосистемы (июнь 2026)

### Существующие кандидаты (нерабочие)

| Крейт | Версия | Описание | Проблема |
|-------|--------|----------|----------|
| [`oq`](https://crates.io/crates/oq) | 0.1.0 | «Object Query — jq for JSON, YAML, TOML, and TOON» | Не компилируется: 10 lifetime-ошибок в jaq-json |
| [`toon_ql`](https://crates.io/crates/toon_ql) | 0.0.2 | «A query language for Toon data» | Только библиотека, 0% доков, нет CLI |

### Экосистема TOON

Все крейты (~180) — конвертеры/парсеры. Ни одного query-инструмента.

Официальный сайт (toonformat.dev) — только CLI-конвертер, playground, подсветка синтаксиса.

### Почему jaq subprocess, а не библиотека

| Подход | Статус |
|--------|--------|
| jaq-core v3.0 как либа | Слишком абстрактный API (DataT, ValT, lifetimes). Не для простого использования. |
| jaq-core v2.x + jaq-json | Совместимы, но нужна конвертация Val ↔ serde_json. Добавляет сложность. |
| `jaq` binary subprocess | ✅ Работает. `-c` даёт компактный вывод (одна строка на значение). Никакой борьбы с API. |

## Текущий API

```bash
# Инспекция
toonq data.toon                    # pretty-print
toonq --head 5 data.toon           # первые N записей
toonq --tail 3 data.toon           # последние N
toonq --count data.toon            # количество записей
toonq --schema data.toon           # поля и типы

# Запросы (полный jq-синтаксис через jaq)
toonq -f '.[] | select(.close > 1400)' data.toon
toonq -f 'sort_by(-.close) | .[0:3]' data.toon

# Формат вывода
toonq --to json data.toon          # TOON → JSON

# Stdin/stdout
cat data.toon | toonq -f '.[0:5]'
toonq -f '.[]' data.toon | toonq --head 3
```
