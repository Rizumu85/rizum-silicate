#ifndef PackageDir
  #error PackageDir must point to the complete staged Windows package
#endif
#ifndef OutputDir
  #error OutputDir is required
#endif
#ifndef AppVersion
  #error AppVersion is required
#endif

[Setup]
AppId={{B58D219B-21F1-4C15-98F8-C7B0C64D713A}
AppName=Rizum Silicate
AppVersion={#AppVersion}
AppPublisher=Rizum
AppPublisherURL=https://github.com/Rizumu85/rizum-silicate
DefaultDirName={localappdata}\Programs\Rizum Silicate
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
; Explorer's COM provider must match the native host architecture.
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
MinVersion=10.0
OutputDir={#OutputDir}
OutputBaseFilename=Rizum-Silicate-windows-x86_64-setup
SetupIconFile=..\..\assets\favicon.ico
UninstallDisplayIcon={app}\Rizum Silicate.exe
LicenseFile={#PackageDir}\LICENSE.txt
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ChangesAssociations=yes
CloseApplications=yes
CloseApplicationsFilter=*.exe,*.dll
RestartApplications=no
SetupLogging=yes

[Files]
Source: "{#PackageDir}\Rizum Silicate.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\rizum_silicate_thumb.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#PackageDir}\tools\*"; DestDir: "{app}\tools"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{userprograms}\Rizum Silicate"; Filename: "{app}\Rizum Silicate.exe"

[Run]
Filename: "{app}\Rizum Silicate.exe"; Description: "Open Rizum Silicate"; Flags: nowait postinstall skipifsilent; Check: IntegrationReady

[Code]
var
  RegistrationSucceeded: Boolean;

function IntegrationReady: Boolean;
begin
  Result := RegistrationSucceeded;
end;

function GetCustomSetupExitCode: Integer;
begin
  if RegistrationSucceeded then Result := 0 else Result := 1;
end;

procedure ApplyIntegration(Action: String);
var
  ResultCode: Integer;
begin
  // Registry ownership stays in Rust, shared with the Settings repair action.
  if not Exec(ExpandConstant('{app}\Rizum Silicate.exe'),
      '--windows-integration ' + Action, ExpandConstant('{app}'),
      SW_HIDE, ewWaitUntilTerminated, ResultCode) then
    RaiseException('Could not start Windows registration. Re-run Setup to repair the application.');
  Log(Format('Windows integration %s returned %d', [Action, ResultCode]));
  if ResultCode <> 0 then
    RaiseException('Windows registration failed. Re-run Setup to repair the application.');
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    ApplyIntegration('install');
    RegistrationSucceeded := True;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  // Run after confirmation, while the owning executable still exists.
  if CurUninstallStep = usUninstall then
    ApplyIntegration('uninstall');
end;
