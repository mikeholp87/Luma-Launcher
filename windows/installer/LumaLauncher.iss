; Per-user installer for the Windows launcher. Build with:
;   dotnet publish windows/native/LumaLauncher.csproj -c Release -r win-x64 --self-contained true -o dist/LumaLauncher
;   iscc /DAppVersion=0.1.5 windows/installer/LumaLauncher.iss
#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{11E29764-09F0-4317-8396-38547683D6B5}
AppName=Luma Launcher
AppVersion={#AppVersion}
AppPublisher=Luma Launcher contributors
AppPublisherURL=https://github.com/mikeholp87/Luma-Launcher
DefaultDirName={autopf}\LumaLauncher
DefaultGroupName=Luma Launcher
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\..\dist
OutputBaseFilename=LumaLauncher-{#AppVersion}-x64-setup
SetupIconFile=..\native\Assets\LumaLauncher.ico
UninstallDisplayIcon={app}\LumaLauncher.exe
LicenseFile=..\..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Files]
Source: "..\..\dist\LumaLauncher\*"; DestDir: "{app}"; Flags: recursesubdirs ignoreversion

[Icons]
Name: "{userprograms}\Luma Launcher"; Filename: "{app}\LumaLauncher.exe"

[Run]
Filename: "{app}\LumaLauncher.exe"; Description: "Start Luma Launcher"; Flags: nowait postinstall skipifsilent
