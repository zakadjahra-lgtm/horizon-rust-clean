; Horizon Rust Clean — Inno Setup script
; البناء: ISCC.exe /DAppVersion=0.1.0 /DSourceExe=..\target\release\hrc-cli.exe horizon.iss

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\target\release\hrc-cli.exe"
#endif

[Setup]
AppId={{6B1C5E52-7D3A-4C8E-9F0B-2A4D8E1C7F31}
AppName=Horizon Rust Clean
AppVersion={#AppVersion}
AppPublisher=Horizon Rust Clean
DefaultDirName={autopf}\Horizon Rust Clean
DefaultGroupName=Horizon Rust Clean
DisableProgramGroupPage=yes
OutputDir=Output
OutputBaseFilename=HorizonRustClean-Setup
Compression=lzma2
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
WizardStyle=modern
UninstallDisplayName=Horizon Rust Clean

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"
Name: "addtopath"; Description: "Add hrc-cli to PATH (use it from any terminal)"; GroupDescription: "Command line:"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\STATUS.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; النسخة الحالية سطر أوامر: الاختصار يفتح نافذة طرفية في مجلد التثبيت ويبقيها مفتوحة
Name: "{autoprograms}\Horizon Rust Clean"; Filename: "{cmd}"; Parameters: "/k ""cd /d ""{app}"" && hrc-cli.exe"""; WorkingDir: "{app}"
Name: "{autodesktop}\Horizon Rust Clean"; Filename: "{cmd}"; Parameters: "/k ""cd /d ""{app}"" && hrc-cli.exe"""; WorkingDir: "{app}"; Tasks: desktopicon

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
