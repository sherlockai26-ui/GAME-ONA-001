@echo off
setlocal

set "ROOT=%~dp0"
set "GAME_EXE=%ROOT%target\release\game-ona-001.exe"
set "TEST_PORT=47191"

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
 "$listener=[System.Net.Sockets.TcpListener]::new([Net.IPAddress]::Parse('127.0.0.1'),%TEST_PORT%); $listener.Start(); Write-Host '[Bridge] Esperando juego en 127.0.0.1:%TEST_PORT%'; $client=$listener.AcceptTcpClient(); $writer=[System.IO.StreamWriter]::new($client.GetStream()); $writer.AutoFlush=$true; $events=@('{\"kind\":\"Joystick\",\"playerId\":1,\"x\":0.75,\"y\":-0.25}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"A\",\"state\":\"down\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"B\",\"state\":\"pressed\"}','{\"kind\":\"Joystick\",\"playerId\":1,\"x\":-0.40,\"y\":0.90}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"A\",\"state\":\"up\"}','{\"kind\":\"Button\",\"playerId\":1,\"button\":\"B\",\"state\":\"released\"}'); while($client.Connected){ foreach($e in $events){ $writer.WriteLine($e); Start-Sleep -Milliseconds 700 } }; $writer.Dispose(); $client.Dispose(); $listener.Stop()"

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
