; MClauncher — Inno Setup script
; البناء: ISCC.exe /DAppExe=..\target\release\horizon-rust-clean.exe /DCliExe=..\target\release\hrc-cli.exe horizon.iss

#ifndef AppVersion
  #define AppVersion "1.0.0"
#endif
#ifndef AppExe
  #define AppExe "..\target\release\horizon-rust-clean.exe"
#endif
#ifndef AgentJar
  #define AgentJar "..\target\release\horizon-client.jar"
#endif
#ifndef RustGame
  #define RustGame "..\target\release\horizon-rust-game.exe"
#endif
#ifndef CliExe
  #define CliExe "..\target\release\hrc-cli.exe"
#endif

[Setup]
AppId={{6B1C5E52-7D3A-4C8E-9F0B-2A4D8E1C7F31}
AppName=MClauncher
AppVersion={#AppVersion}
AppPublisher=MClauncher
DefaultDirName={autopf}\MClauncher
DefaultGroupName=MClauncher
DisableProgramGroupPage=yes
OutputDir=Output
OutputBaseFilename=MClauncher-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
WizardStyle=modern
SetupIconFile=..\assets\icon.ico
UninstallDisplayIcon={app}\horizon-rust-clean.exe
UninstallDisplayName=MClauncher

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"
Name: "addtopath"; Description: "Add hrc-cli to PATH (use it from any terminal)"; GroupDescription: "Command line:"; Flags: unchecked

[Files]
Source: "{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#CliExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#AgentJar}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#RustGame}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\icon.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\STATUS.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\MClauncher"; Filename: "{app}\horizon-rust-clean.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\MClauncher"; Filename: "{app}\horizon-rust-clean.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
Filename: "{app}\horizon-rust-clean.exe"; Description: "Launch MClauncher"; Flags: nowait postinstall skipifsilent

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Code]
function NeedsAddPath(Param: string): Boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + Lowercase(Param) + ';', ';' + Lowercase(OrigPath) + ';') = 0;
end;
