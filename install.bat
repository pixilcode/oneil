@echo off
setlocal EnableExtensions

rem Directory containing this script (trailing backslash)
set "SCRIPT_DIR=%~dp0"
set "NO_PYTHON=0"
set "EDITABLE=0"

:arg_loop
if "%~1"=="" goto arg_done
if /i "%~1"=="--no-python" (
  set "NO_PYTHON=1"
  shift
  goto arg_loop
)
if /i "%~1"=="-e" (
  set "EDITABLE=1"
  shift
  goto arg_loop
)
if /i "%~1"=="--editable" (
  set "EDITABLE=1"
  shift
  goto arg_loop
)
if /i "%~1"=="-h" goto show_help
if /i "%~1"=="--help" goto show_help
echo Unknown option: %~1
echo Run install.bat --help
exit /b 1

:show_help
call :usage
exit /b 0

:arg_done
if "%NO_PYTHON%"=="1" if "%EDITABLE%"=="1" (
  echo Note: --editable has no effect with --no-python.
)

where cargo >nul 2>&1
if errorlevel 1 (
  echo Error: Cargo was not found on your PATH.
  echo.
  echo Install the Rust toolchain with rustup:
  echo   https://rustup.rs/
  echo.
  echo Download and run rustup-init.exe from the site above.
  echo.
  echo Then restart this terminal and ensure %%USERPROFILE%%\.cargo\bin is on your PATH.
  exit /b 1
)

set "ONEIL_PKG=%SCRIPT_DIR%src-rs\oneil"
if not exist "%ONEIL_PKG%\Cargo.toml" (
  echo Error: expected Cargo.toml at "%ONEIL_PKG%"
  exit /b 1
)

echo Installing Oneil CLI with Cargo...
if "%NO_PYTHON%"=="1" (
  cargo install --force --path "%ONEIL_PKG%" --no-default-features --features rust-lib
) else (
  cargo install --force --path "%ONEIL_PKG%"
)
if errorlevel 1 exit /b 1

if "%NO_PYTHON%"=="1" goto finish

if not exist "%SCRIPT_DIR%pyproject.toml" (
  echo Error: pyproject.toml not found at "%SCRIPT_DIR%"
  exit /b 1
)

set "PYTHON_CMD="
where py >nul 2>&1
if not errorlevel 1 (
  py -3 -c "import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)" >nul 2>&1
  if not errorlevel 1 set "PYTHON_CMD=py -3"
)
if not defined PYTHON_CMD (
  where python >nul 2>&1
  if not errorlevel 1 (
    python -c "import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)" >nul 2>&1
    if not errorlevel 1 set "PYTHON_CMD=python"
  )
)
if not defined PYTHON_CMD (
  echo Error: Python 3.10+ is required for the library install but no suitable Python was found.
  echo Install Python 3.10 or newer, or re-run with --no-python to install only the CLI with no Python bindings.
  exit /b 1
)

echo Installing Oneil Python package...
pushd "%SCRIPT_DIR%"
if "%EDITABLE%"=="1" (
  %PYTHON_CMD% -m pip install -e .
) else (
  %PYTHON_CMD% -m pip install .
)
if errorlevel 1 (
  popd
  exit /b 1
)
popd

:finish
echo.
echo Done.
echo Ensure %%USERPROFILE%%\.cargo\bin is on your PATH to run: oneil --version
exit /b 0

:usage
echo Usage: install.bat [options]
echo.
echo   Builds and installs the Oneil CLI with Cargo. By default, also installs the
echo   Python package ^(import oneil^) for the current interpreter.
echo.
echo Options:
echo   --no-python    Install the CLI only: no Python bindings and no pip package.
echo   -e, --editable Install the Python package in editable mode ^(development^).
echo   -h, --help     Show this help.
echo.
echo Prerequisites:
echo   - Cargo ^(Rust^): https://rustup.rs/
echo   - For the default install: Python 3.10+ with pip
exit /b 0
