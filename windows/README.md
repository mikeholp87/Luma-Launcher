# Windows preview

Luma's Windows preview uses Windows PowerShell 5.1 and WPF, included with
supported desktop editions of Windows. It does not require Rust, GTK, or
administrator access. Keep the files in this directory together and
double-click `LumaLauncher.cmd`, or run:

```powershell
powershell.exe -NoProfile -STA -ExecutionPolicy Bypass -File .\windows\LumaLauncher.ps1
```

Type to search the Start Menu, use the arrow keys and Enter to launch a result,
or double-click it. Escape hides the palette; Ctrl+Alt+Space toggles it when
available. If the shortcut is unavailable, launching Luma again toggles the
existing instance. F5 refreshes the index. Use the quit button in the search
bar or Alt+F4 to exit. The script also accepts `-Background`, `-Toggle`, and
`-Version`.

| Command | Behavior |
|---|---|
| `LumaLauncher.cmd` | Show the launcher, or toggle the running instance |
| `LumaLauncher.cmd -Background` | Start hidden with the global shortcut registered |
| `LumaLauncher.cmd -Toggle` | Toggle the existing instance, or start and show it |
| `powershell.exe -NoProfile -File .\windows\LumaLauncher.ps1 -Version` | Print the version to the terminal |

The CMD wrapper starts PowerShell asynchronously, so use the direct PowerShell
command when you need terminal output. Include `-ExecutionPolicy Bypass` in that
command if the local execution policy prevents running scripts.

The dark palette follows the Linux interface, including the search bar,
highlighted results, and keyboard hints. Exact and prefix matches rank ahead of
substring and fuzzy matches. Favorites and launch frequency improve ordering
within a match tier. Ctrl+D pins or unpins an app, Ctrl+Delete forgets its
launch history, Ctrl+J/K navigate, and Ctrl+, opens Settings. Settings can
disable history or fuzzy matching and clear history.

Enter `= (250 + 75) * 2` for a calculation; Enter copies the result. Prefix a
query with `g ` or `ddg ` to search the web, or enter an HTTP/HTTPS URL to open
it. URLs open through the Windows default browser. History, favorites, and
settings are stored locally in `%LOCALAPPDATA%\LumaLauncher\history.json`.
If that file is damaged, Luma preserves it and stops writing to it.
Disabling history stops recording new launches; use Settings to clear existing
history. Favorites are stored separately in the same file and are not cleared by
that button. Luma does not collect telemetry or track applications launched
outside the launcher.

The preview searches `.lnk` files in the user's and common Start Menu Programs
folders. Apps without shortcuts there, including some Microsoft Store apps,
may be missing. Windows-specific integration does not yet include system usage
statistics, automatic login startup, app icons, Linux desktop actions, theme or
layout customization, or Linux portal shortcuts. Windows may restrict focus
when the palette is activated by another process.

## Troubleshooting and removal

- If Ctrl+Alt+Space is unavailable, another app may have registered it. Start
  the launcher again to toggle its window, or release the conflicting shortcut.
- If an app is missing, confirm it has a `.lnk` shortcut in one of the Start
  Menu Programs folders, then press F5.
- If history cannot be loaded, repair or move the named `history.json` file
  after exiting Luma. The launcher will not overwrite a damaged file.
- If Windows blocks the scripts, check your execution policy and organization
  policy. The included CMD file requests a process-only bypass.

To remove the preview, quit it and remove the `windows/` directory. To also
remove preferences, favorites, and history, delete the `LumaLauncher` directory
under `%LOCALAPPDATA%` separately.

Run the core tests with:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\windows\Test-Core.ps1
```

`-ExecutionPolicy Bypass` applies only to the launched PowerShell process; it
does not change the machine's execution policy. Group Policy may prevent script
execution.
