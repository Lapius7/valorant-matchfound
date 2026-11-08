#ifndef AppVersion
  #define AppVersion "0.0.8"
#endif
#define AppName "valorant-matchfound"

[Setup]
AppId={{B7C1D0E4-5A2F-4E6B-9C38-7D1F0A9E2B64}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=Lapius
DefaultDirName={localappdata}\Programs\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
WizardStyle=modern
CloseApplications=force
RestartApplications=no
OutputDir=Output
OutputBaseFilename={#AppName}-installer-v{#AppVersion}
Compression=lzma2
SolidCompression=yes
UninstallDisplayIcon={app}\{#AppName}.exe
SetupIconFile=..\src-tauri\icons\icon.ico
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Files]
Source: "..\src-tauri\target\x86_64-pc-windows-gnu\release\{#AppName}.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\src-tauri\target\x86_64-pc-windows-gnu\release\WebView2Loader.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppName}.exe"

[Run]
; 通常のインストール: 完了画面のチェックから起動
Filename: "{app}\{#AppName}.exe"; Description: "{#AppName} を起動する"; Flags: nowait postinstall skipifsilent

[Code]
// 自動アップデート(サイレント)の場合は、インストール後にウィンドウを表示して再起動する
procedure CurStepChanged(CurStep: TSetupStep);
var Code: Integer;
begin
  if (CurStep = ssPostInstall) and WizardSilent then
    Exec(ExpandConstant('{app}\{#AppName}.exe'), '', ExpandConstant('{app}'), SW_SHOW, ewNoWait, Code);
end;

// 起動中のアプリがあれば、確認なしで閉じてからインストールする(トレイ常駐で閉じにくいため)
function InitializeSetup(): Boolean;
var Code: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppName}.exe', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Sleep(600);
  Result := True;
end;
