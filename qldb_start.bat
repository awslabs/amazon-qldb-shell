echo off
REM Initiate a cmd, go to user's home path and run qldb.exe help
cmd /Q /K "cd %HOMEPATH% && qldb.exe -h"