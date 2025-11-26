#!/bin/bash

# Продвинутый скрипт для работы с Cargo проектами

set -e

SCRIPT_NAME="cargo_build.sh"

# Цвета для вывода
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Функции для цветного вывода
error() { echo -e "${RED}❌ $1${NC}"; }
success() { echo -e "${GREEN}✅ $1${NC}"; }
warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
info() { echo -e "${BLUE}🔧 $1${NC}"; }
debug() { echo -e "🐛 $1"; }

show_help() {
  echo "Использование: ./$SCRIPT_NAME [COMMAND]"
  echo ""
  echo "COMMANDS:"
  echo "  run, r       Сборка и запуск (по умолчанию)"
  echo "  build, b     Только сборка"
  echo "  check, c     Проверка кода без сборки"
  echo "  test, t      Запуск тестов"
  echo "  clean, cl    Очистка проекта"
  echo "  release, rel Release сборка и запуск"
  echo "  doc, d       Генерация документации"
  echo "  new, n       Создать новый проект"
  echo "  help, h      Показать эту справку"
  echo ""
  echo "OPTIONS:"
  echo "  --verbose    Подробный вывод"
  echo ""
  echo "Примеры:"
  echo "  ./$SCRIPT_NAME           # Сборка и запуск"
  echo "  ./$SCRIPT_NAME test      # Запуск тестов"
  echo "  ./$SCRIPT_NAME release   # Release сборка"
}

# Переменные
VERBOSE=false
COMMAND="run"

# Парсинг аргументов
while [[ $# -gt 0 ]]; do
  case $1 in
  run | r)
    COMMAND="run"
    shift
    ;;
  build | b)
    COMMAND="build"
    shift
    ;;
  check | c)
    COMMAND="check"
    shift
    ;;
  test | t)
    COMMAND="test"
    shift
    ;;
  clean | cl)
    COMMAND="clean"
    shift
    ;;
  release | rel)
    COMMAND="release"
    shift
    ;;
  doc | d)
    COMMAND="doc"
    shift
    ;;
  new | n)
    COMMAND="new"
    shift
    ;;
  --verbose)
    VERBOSE=true
    shift
    ;;
  help | h | -h | --help)
    show_help
    exit 0
    ;;
  *)
    error "Неизвестная команда: $1"
    show_help
    exit 1
    ;;
  esac
done

# Функция для выполнения cargo команд с проверкой
run_cargo() {
  local cmd=$1
  local message=$2

  info "$message"

  if [ "$VERBOSE" = true ]; then
    cargo $cmd
  else
    cargo $cmd --quiet
  fi

  if [ $? -eq 0 ]; then
    success "Команда '$cmd' выполнена успешно"
  else
    error "Ошибка выполнения '$cmd'"
    exit 1
  fi
}

# Обработка команд
case $COMMAND in
"run")
  if [ ! -f "Cargo.toml" ]; then
    error "Cargo.toml не найден!"
    echo "💡 Используйте: ./$SCRIPT_NAME new"
    exit 1
  fi
  run_cargo "run" "Сборка и запуск проекта..."
  ;;

"build")
  if [ ! -f "Cargo.toml" ]; then
    error "Cargo.toml не найден!"
    exit 1
  fi
  run_cargo "build" "Сборка проекта..."
  info "Бинарник: target/debug/$(basename $(pwd))"
  ;;

"check")
  if [ ! -f "Cargo.toml" ]; then
    error "Cargo.toml не найден!"
    exit 1
  fi
  run_cargo "check" "Проверка кода..."
  ;;

"test")
  if [ ! -f "Cargo.toml" ]; then
    error "Cargo.toml не найден!"
    exit 1
  fi
  run_cargo "test" "Запуск тестов..."
  ;;

"clean")
  if [ ! -f "Cargo.toml" ]; then
    error "Cargo.toml не найден!"
    exit 1
  fi
  run_cargo "clean" "Очистка проекта..."
  ;;

"release")
  if [ ! -f "Cargo.toml" ]; then
    error "Cargo.toml не найден!"
    exit 1
  fi
  info "Release сборка и запуск..."
  if [ "$VERBOSE" = true ]; then
    cargo run --release
  else
    cargo run --release --quiet
  fi
  ;;

"doc")
  if [ ! -f "Cargo.toml" ]; then
    error "Cargo.toml не найден!"
    exit 1
  fi
  run_cargo "doc" "Генерация документации..."
  info "Документация: target/doc/$(basename $(pwd))/index.html"
  ;;

"new")
  echo "💡 Введите имя нового проекта:"
  read -r project_name
  if [ -n "$project_name" ]; then
    cargo new "$project_name"
    success "Проект '$project_name' создан"
    info "Перейдите в директорию: cd $project_name"
  else
    error "Имя проекта не может быть пустым"
  fi
  ;;
esac
