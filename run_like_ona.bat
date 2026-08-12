@echo off
setlocal

set "ROOT=%~dp0"
set "GAME_EXE=%ROOT%target\release\game-ona-001.exe"
set "INPUT_PORT=47191"
set "LIFECYCLE_PORT=48191"

echo [GAME ONA 001] MOCK / DEVELOPMENT ONLY
echo [GAME ONA 001] This does not use the real ONA Gaming Display or ONA bridges.
echo.

if not exist "%GAME_EXE%" (
    echo [GAME ONA 001] No existe el ejecutable release. Compilando...
    pushd "%ROOT%" >nul
    cargo build --release
    if errorlevel 1 (
        popd >nul
        echo [GAME ONA 001] ERROR: fallo la compilacion.
        pause
        exit /b 1
    )
    popd >nul
)

for /f "tokens=1-4 delims=," %%a in ('powershell -NoProfile -Command "Add-Type -AssemblyName System.Windows.Forms; $s=[System.Windows.Forms.Screen]::PrimaryScreen; '{0},{1},{2},{3}' -f $s.Bounds.X,$s.Bounds.Y,$s.Bounds.Width,$s.Bounds.Height"') do (
    set "DISPLAY_X=%%a"
    set "DISPLAY_Y=%%b"
    set "DISPLAY_WIDTH=%%c"
    set "DISPLAY_HEIGHT=%%d"
)

echo [GAME ONA 001] Mock display: %DISPLAY_X%,%DISPLAY_Y% %DISPLAY_WIDTH%x%DISPLAY_HEIGHT%

echo [GAME ONA 001] Iniciando Input Bridge falso en 127.0.0.1:%INPUT_PORT%...
start "GAME ONA 001 Input Bridge MOCK" /min powershell -NoProfile -ExecutionPolicy Bypass -Command ^
 "$listener=[System.Net.Sockets.TcpListener]::new([Net.IPAddress]::Parse('127.0.0.1'),%INPUT_PORT%); $listener.Start(); Write-Host '[Input Bridge MOCK] Development only'; $client=$listener.AcceptTcpClient(); $writer=[System.IO.StreamWriter]::new($client.GetStream()); $writer.AutoFlush=$true; $events=@('{\"kind\":\"Joystick\",\"playerId\":1,\"x\":0.75,\"y\":-0.25}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"A\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"A\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"B\",\"state\":\"pressed\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"B\",\"state\":\"released\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"X\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"X\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Y\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Y\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L1\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L1\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L2\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L2\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R1\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R1\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R2\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R2\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Select\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Select\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Start\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Start\",\"state\":\"up\"}'); while($client.Connected){ foreach($e in $events){ $writer.WriteLine($e); Start-Sleep -Milliseconds 350 } }; $writer.Dispose(); $client.Dispose(); $listener.Stop()"

echo [GAME ONA 001] Iniciando Lifecycle Bridge falso en 127.0.0.1:%LIFECYCLE_PORT%...
start "GAME ONA 001 Lifecycle Bridge MOCK" /min powershell -NoProfile -ExecutionPolicy Bypass -Command ^
 "$listener=[System.Net.Sockets.TcpListener]::new([Net.IPAddress]::Parse('127.0.0.1'),%LIFECYCLE_PORT%); $listener.Start(); Write-Host '[Lifecycle Bridge MOCK] Development only'; $client=$listener.AcceptTcpClient(); $reader=[System.IO.StreamReader]::new($client.GetStream()); while($client.Connected){ $line=$reader.ReadLine(); if($null -eq $line){ break }; Write-Host ('[Lifecycle] ' + $line) }; $reader.Dispose(); $client.Dispose(); $listener.Stop()"

timeout /t 1 /nobreak >nul

set "ONA_RUNTIME=1"
set "ONA_PROTOCOL_VERSION=1"
set "ONA_INPUT_HOST=127.0.0.1"
set "ONA_INPUT_PORT=%INPUT_PORT%"
set "ONA_LIFECYCLE_HOST=127.0.0.1"
set "ONA_LIFECYCLE_PORT=%LIFECYCLE_PORT%"
set "ONA_PLAYER_ID=1"
set "ONA_DISPLAY_ID=mock-primary"
set "ONA_DISPLAY_TARGET=mock-development"
set "ONA_DISPLAY_NAME=MOCK PRIMARY DISPLAY"
set "ONA_DISPLAY_X=%DISPLAY_X%"
set "ONA_DISPLAY_Y=%DISPLAY_Y%"
set "ONA_DISPLAY_WIDTH=%DISPLAY_WIDTH%"
set "ONA_DISPLAY_HEIGHT=%DISPLAY_HEIGHT%"
set "ONA_DISPLAY_SCALE_FACTOR=1.0"
set "ONA_DISPLAY_MODE=WINDOWED"

echo [GAME ONA 001] Ejecutando como ONA Runtime mock...
pushd "%ROOT%" >nul
"%GAME_EXE%"
popd >nul

endlocal
