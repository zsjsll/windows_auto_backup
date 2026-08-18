' ============================================================
' Run backup with a VISIBLE console window (shows colored log).
'   1 = normal window    True = wait for the process to exit
' The exe declares requireAdministrator, so UAC will prompt.
' NOTE: Keep this file in ASCII to avoid code-page issues.
' ============================================================
Option Explicit

Dim shell, fso, exePath, exeDir, scriptDir
Set shell = CreateObject("WScript.Shell")
Set fso = CreateObject("Scripting.FileSystemObject")

scriptDir = fso.GetParentFolderName(WScript.ScriptFullName)
' Script lives in deploy\vbs\ while the exe lives one level up in deploy\
exeDir = fso.GetParentFolderName(scriptDir)
exePath = fso.BuildPath(exeDir, "rust_snapshot_backup.exe")

If Not fso.FileExists(exePath) Then
    MsgBox "Executable not found: " & exePath, vbCritical, "Startup failed"
    WScript.Quit 1
End If

' Switch working directory to the exe folder so the program can find
' its relative paths (config, logs, bin, ...)
shell.CurrentDirectory = exeDir

shell.Run """" & exePath & """", 1, True
