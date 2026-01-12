@echo off
REM Test script to run multiple peers locally (Windows)

echo Starting 3 local peers for testing...
echo.

REM Peer 1: Headless service + TUI in separate windows
echo Starting Peer 1 service (ports 9000-9010) - minimized...
start "Peer 1 Service" /MIN /D "%CD%" cmd /c "set DATABASE_LIBSQL_PATH=shared/data/peer1.db && set UPPE_KEYPAIR_PATH=peer1_keypair.key && cargo run --bin uppe-service -- --config apps/service/test-peer1.toml run"

timeout /t 3 /nobreak >nul

echo Starting Peer 1 TUI - Green window...
start "Uppe Peer 1 - TUI" /D "%CD%" cmd /t:0A /k "mode con: cols=120 lines=35 && set DATABASE_LIBSQL_PATH=shared/data/peer1.db && set UPPE_KEYPAIR_PATH=peer1_keypair.key && cargo run --bin uppe-service -- --config apps/service/test-peer1.toml tui"

timeout /t 2 /nobreak >nul

REM Peer 2: Headless service + TUI in separate windows
echo Starting Peer 2 service (ports 9100-9110) - minimized...
start "Peer 2 Service" /MIN /D "%CD%" cmd /c "set DATABASE_LIBSQL_PATH=shared/data/peer2.db && set UPPE_KEYPAIR_PATH=peer2_keypair.key && cargo run --bin uppe-service -- --config apps/service/test-peer2.toml run"

timeout /t 3 /nobreak >nul

echo Starting Peer 2 TUI - Cyan window...
start "Uppe Peer 2 - TUI" /D "%CD%" cmd /t:0B /k "mode con: cols=120 lines=35 && set DATABASE_LIBSQL_PATH=shared/data/peer2.db && set UPPE_KEYPAIR_PATH=peer2_keypair.key && cargo run --bin uppe-service -- --config apps/service/test-peer2.toml tui"

timeout /t 2 /nobreak >nul

REM Peer 3: Just headless service with logs
echo Starting Peer 3 headless (ports 9200-9210) - Red window...
start "Uppe Peer 3 - Headless" /D "%CD%" cmd /t:0C /k "mode con: cols=100 lines=30 && set DATABASE_LIBSQL_PATH=shared/data/peer3.db && set UPPE_KEYPAIR_PATH=peer3_keypair.key && cargo run --bin uppe-service -- --config apps/service/test-peer3.toml run"

echo.
echo All 3 peers started!
echo.
echo VISIBLE WINDOWS (3):
echo   Green: Peer 1 TUI dashboard (reads from peer1.db)
echo   Cyan: Peer 2 TUI dashboard (reads from peer2.db)
echo   Red: Peer 3 raw logs (ports 9200-9210)
echo.
echo BACKGROUND SERVICES (2 minimized):
echo   Peer 1 service running (ports 9000-9010)
echo   Peer 2 service running (ports 9100-9110)
echo.
echo The TUI windows show real-time data from the background services.
echo.
echo mDNS will help them discover each other automatically.
echo Watch the TUI for 'Peer connected' events and P2P stats!
echo.
echo IMPORTANT: Close TUI windows first, then close minimized service windows!
echo.
pause


