# TODO.md — toonq

## Этап 1: Минимальный CLI ✅

- [x] Инициализировать Cargo-проект
- [x] CLI: clap-парсер аргументов
- [x] TOON → Value через serde_toon_format
- [x] Применить jaq-фильтр (через jaq subprocess с -c)
- [x] Вывод TOON / JSON
- [x] Stdin/stdout

## Этап 2: Инспекция ✅

- [x] --head N / --tail N
- [x] --schema
- [x] --count

## Этап 3: Полировка ✅

- [x] --from json / авто-детект формата (.toon/.json)
- [x] --stats (статистика токенов, TOON vs JSON)
- [x] --to raw (вывод jaq без TOON-обёртки)
- [x] 11 тестов на реальных данных
- [x] Конвертация формата как самостоятельная операция

## Бэклог (будущее)

- [ ] Интеграция с mk (рецепт в mkfile invest-research)
- [ ] `--color` — цветной вывод
- [ ] `--compact` / `-c` — компактный TOON (без pretty-print)
- [ ] `install` — cargo install toonq
- [ ] CI: тесты в GitHub Actions
