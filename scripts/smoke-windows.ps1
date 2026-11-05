<#
.SYNOPSIS
  Test de fumée de l'installeur NSIS Windows.

.DESCRIPTION
  1. Vérifie que le manifeste de l'installeur demande « asInvoker » (pas d'invite UAC, N1).
  2. Installe en mode silencieux (/S) pour l'utilisateur courant.
  3. Vérifie le dossier d'installation, le raccourci du menu Démarrer et
     l'entrée de désinstallation dans HKCU (Paramètres Windows).
  4. Lance Arnod, vérifie la fenêtre « Arnod », puis la ferme comme avec la croix (F4).
  5. Désinstalle en mode silencieux et vérifie la suppression.

  Limite : le runner CI est administrateur. L'absence d'invite UAC sur un compte
  standard se vérifie à la main (voir README), ce script contrôle le manifeste.

.EXAMPLE
  pwsh scripts/smoke-windows.ps1 -Installer .\src-tauri\target\release\bundle\nsis\Arnod_0.1.0_x64-setup.exe
#>
param(
  [Parameter(Mandatory)] [string] $Installer
)

$ErrorActionPreference = 'Stop'

function Wait-Until([scriptblock] $Condition, [int] $TimeoutSeconds, [string] $Message) {
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while (-not (& $Condition)) {
    if ((Get-Date) -gt $deadline) { throw $Message }
    Start-Sleep -Milliseconds 500
  }
}

$installDir = Join-Path $env:LOCALAPPDATA 'Arnod'
$exe = Join-Path $installDir 'arnod.exe'
$uninstaller = Join-Path $installDir 'uninstall.exe'
$shortcut = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Arnod.lnk'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Arnod'

# 1. Niveau d'exécution demandé par l'installeur.
$bytes = [System.IO.File]::ReadAllBytes($Installer)
$text = [System.Text.Encoding]::ASCII.GetString($bytes)
if ($text -notmatch 'requestedExecutionLevel\s+level="asInvoker"') {
  throw "Le manifeste de l'installeur ne demande pas « asInvoker » : une invite UAC est possible."
}
Write-Host "Manifeste : requestedExecutionLevel = asInvoker (pas d'élévation)."

# 2. Installation silencieuse.
$setup = Start-Process -FilePath $Installer -ArgumentList '/S' -Wait -PassThru
if ($setup.ExitCode -ne 0) { throw "L'installeur s'est terminé avec le code $($setup.ExitCode)." }

# 3. Emplacement, raccourci et entrée de désinstallation.
if (-not (Test-Path $exe)) { throw "arnod.exe introuvable dans $installDir." }
if (-not (Test-Path $shortcut)) { throw "Raccourci du menu Démarrer introuvable : $shortcut." }
if (-not (Test-Path $uninstallKey)) { throw "Entrée de désinstallation absente de HKCU." }
$location = (Get-ItemProperty $uninstallKey).InstallLocation.Trim('"')
Write-Host "Installé dans $location (entrée de désinstallation dans HKCU)."

# 4. Lancement puis fermeture par la croix (WM_CLOSE sur la fenêtre principale).
$app = Start-Process -FilePath $exe -PassThru
Wait-Until -TimeoutSeconds 30 -Message "La fenêtre « Arnod » n'est pas apparue." -Condition {
  if ($app.HasExited) { throw "Arnod s'est arrêté au démarrage (code $($app.ExitCode))." }
  $app.Refresh()
  $app.MainWindowTitle -eq 'Arnod'
}
Write-Host "Fenêtre « Arnod » ouverte (PID $($app.Id))."
Start-Sleep -Seconds 3

if (-not $app.CloseMainWindow()) { throw "Impossible d'envoyer la fermeture à la fenêtre." }
if (-not $app.WaitForExit(15000)) {
  Stop-Process -Id $app.Id -Force
  throw "Arnod ne s'est pas fermé après la fermeture de la fenêtre."
}
if ($app.ExitCode -ne 0) { throw "Arnod s'est fermé avec le code $($app.ExitCode)." }
Write-Host "Arnod s'est fermé proprement (code 0)."

# 5. Désinstallation silencieuse.
Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait | Out-Null
# L'uninstaller NSIS se recopie dans %TEMP% et rend la main aussitôt : on attend son effet.
Wait-Until -TimeoutSeconds 60 -Message "La désinstallation n'a pas supprimé arnod.exe et l'entrée HKCU." -Condition {
  -not (Test-Path $exe) -and -not (Test-Path $uninstallKey)
}
Write-Host "Désinstallation silencieuse réussie."
