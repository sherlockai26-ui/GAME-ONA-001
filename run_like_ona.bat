@echo off
setlocal

set "ROOT=%~dp0"
set "GAME_EXE=%ROOT%target\release\game-ona-001.exe"
set "TEST_PORT=47191"

echo [GAME ONA 001] MOCK / DEVELOPMENT ONLY
echo [GAME ONA 001] This does not use the real ONA Input Bridge.
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

echo [GAME ONA 001] Iniciando bridge falso en 127.0.0.1:%TEST_PORT%...
start "GAME ONA 001 Test Bridge" /min powershell -NoProfile -ExecutionPolicy Bypass -Command ^
 "$listener=[System.Net.Sockets.TcpListener]::new([Net.IPAddress]::Parse('127.0.0.1'),%TEST_PORT%); $listener.Start(); Write-Host '[Bridge MOCK] Development only - waiting on 127.0.0.1:%TEST_PORT%'; $client=$listener.AcceptTcpClient(); $writer=[System.IO.StreamWriter]::new($client.GetStream()); $writer.AutoFlush=$true; $events=@('{\"kind\":\"Joystick\",\"playerId\":1,\"x\":0.75,\"y\":-0.25}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"A\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"A\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"B\",\"state\":\"pressed\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"B\",\"state\":\"released\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"X\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"X\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Y\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Y\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L1\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L1\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L2\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"L2\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R1\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R1\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R2\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"R2\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Select\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Select\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Start\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"Start\",\"state\":\"up\"}'); while($client.Connected){ foreach($e in $events){ $writer.WriteLine($e); Start-Sleep -Milliseconds 350 } }; $writer.Dispose(); $client.Dispose(); $listener.Stop()"

timeout /t 1 /nobreak >nul

set "ONA_RUNTIME=1"
set "ONA_PROTOCOL_VERSION=1"
set "ONA_INPUT_HOST=127.0.0.1"
set "ONA_INPUT_PORT=%TEST_PORT%"

echo [GAME ONA 001] Ejecutando como ONA Runtime...
pushd "%ROOT%" >nul
"%GAME_EXE%"
popd >nul

endlocal
