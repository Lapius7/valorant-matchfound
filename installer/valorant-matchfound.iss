#ifndef AppVersion
  #define AppVersion "0.0.13"
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
; Windows の表示言語に合うものを選ぶ。合うものが無いときだけ言語の選択画面を出す
ShowLanguageDialog=auto

; アプリと同じ11言語。先頭の英語は、どれにも合わないときの既定
; 中国語は Inno Setup 6.5 に同梱されていないため、installer\languages に置いた非公式訳を使う
[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "korean"; MessagesFile: "compiler:Languages\Korean.isl"
Name: "chinesesimplified"; MessagesFile: "languages\ChineseSimplified.isl"
Name: "chinesetraditional"; MessagesFile: "languages\ChineseTraditional.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"
Name: "german"; MessagesFile: "compiler:Languages\German.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"
Name: "italian"; MessagesFile: "compiler:Languages\Italian.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Files]
Source: "..\src-tauri\target\x86_64-pc-windows-gnu\release\{#AppName}.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\src-tauri\target\x86_64-pc-windows-gnu\release\WebView2Loader.dll"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppName}.exe"

[Run]
; 通常のインストール: 完了画面のチェックから起動
Filename: "{app}\{#AppName}.exe"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent

[Code]
// インストーラーで選ばれた言語を、アプリの初回の言語として渡す(アプリで言語を選んだことがあれば上書きしない)
function AppLang(): String;
begin
  case ActiveLanguage of
    'japanese': Result := 'ja';
    'korean': Result := 'ko';
    'chinesesimplified': Result := 'zh-CN';
    'chinesetraditional': Result := 'zh-TW';
    'russian': Result := 'ru';
    'german': Result := 'de';
    'french': Result := 'fr';
    'italian': Result := 'it';
    'spanish': Result := 'es';
    'brazilianportuguese': Result := 'pt-BR';
  else
    Result := 'en';
  end;
end;

procedure SaveAppLang();
var Dir: String;
begin
  // Tauri の app_data_dir(%APPDATA%\<identifier>)
  Dir := ExpandConstant('{userappdata}\com.lapius7.valorant-matchfound');
  if not FileExists(Dir + '\lang') then
  begin
    ForceDirectories(Dir);
    SaveStringToFile(Dir + '\lang', AppLang(), False);
  end;
end;

// 自動アップデート(サイレント)の場合は、インストール後にウィンドウを表示して再起動する
procedure CurStepChanged(CurStep: TSetupStep);
var Code: Integer;
begin
  if CurStep = ssPostInstall then SaveAppLang();
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
