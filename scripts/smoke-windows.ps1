<#
.SYNOPSIS
  Test de fumée de l'installeur NSIS Windows.

.DESCRIPTION
  1. Vérifie que le manifeste de l'installeur demande « asInvoker » (pas d'invite UAC, N1).
  2. Installe en mode silencieux (/S) pour l'utilisateur courant.
  3. Vérifie le dossier d'installation, le raccourci du menu Démarrer et
     l'entrée de désinstallation dans HKCU (Paramètres Windows).
  4. Lance Arnod, attend que la page affiche « Hello World » (F1, F2, via UI Automation),
     enregistre une capture, puis ferme la fenêtre comme avec la croix (F4).
  5. Désinstalle en mode silencieux et vérifie la suppression.

  Limite : le runner CI est administrateur. L'absence d'invite UAC sur un compte
  standard se vérifie à la main (voir README), ce script contrôle le manifeste.

.EXAMPLE
  pwsh scripts/smoke-windows.ps1 -Installer .\src-tauri\target\release\bundle\nsis\Arnod_0.1.0_x64-setup.exe
#>
param(
  [Parameter(Mandatory)] [string] $Installer,
  # Capture d'écran prise une fois la page rendue (artefact CI).
  [string] $Screenshot = 'smoke-windows.png'
)

$ErrorActionPreference = 'Stop'

function Wait-Until([scriptblock] $Condition, [int] $TimeoutSeconds, [string] $Message) {
  $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
  while (-not (& $Condition)) {
    if ((Get-Date) -gt $deadline) { throw $Message }
    Start-Sleep -Milliseconds 500
  }
}

function Get-WebView2Version {
  $client = 'Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
  foreach ($key in "HKLM:\SOFTWARE\WOW6432Node\$client", "HKCU:\Software\$client") {
    $version = (Get-ItemProperty -Path $key -ErrorAction SilentlyContinue).pv
    if ($version -and $version -ne '0.0.0.0') { return $version }
  }
  return 'absent'
}

function Initialize-UIAutomation {
  try {
    Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
    return $true
  } catch {
    return $false
  }
}

function Find-UIElementByName([IntPtr] $Handle, [string] $Name) {
  if ($Handle -eq [IntPtr]::Zero) { return $null }
  try {
    $automation = [System.Windows.Automation.AutomationElement]
    $root = $automation::FromHandle($Handle)
    $condition = [System.Windows.Automation.PropertyCondition]::new($automation::NameProperty, $Name)
    return $root.FindFirst([System.Windows.Automation.TreeScope]::Descendants, $condition)
  } catch {
    # Arbre d'accessibilité pas encore prêt (WebView2 en cours d'initialisation).
    return $null
  }
}

# Diagnostic uniquement : une capture ratée ne fait pas échouer le test.
function Save-Screenshot([string] $Path) {
  try {
    # .NET ne suit pas le dossier courant de PowerShell : chemin absolu.
    $Path = [System.IO.Path]::GetFullPath((Join-Path (Get-Location) $Path))
    Add-Type -AssemblyName System.Windows.Forms, System.Drawing
    $bounds = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $bitmap = [System.Drawing.Bitmap]::new($bounds.Width, $bounds.Height)
    $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
    $graphics.CopyFromScreen($bounds.Location, [System.Drawing.Point]::Empty, $bounds.Size)
    $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
    $graphics.Dispose()
    $bitmap.Dispose()
    Write-Host "Capture enregistrée : $Path"
  } catch {
    Write-Host "Capture impossible : $($_.Exception.Message)"
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

# 2. Installation silencieuse (télécharge WebView2 s'il manque).
Write-Host "WebView2 avant installation : $(Get-WebView2Version)."
$setup = Start-Process -FilePath $Installer -ArgumentList '/S' -Wait -PassThru
if ($setup.ExitCode -ne 0) { throw "L'installeur s'est terminé avec le code $($setup.ExitCode)." }
Write-Host "WebView2 après installation : $(Get-WebView2Version)."

# 3. Emplacement, raccourci et entrée de désinstallation.
if (-not (Test-Path $exe)) { throw "arnod.exe introuvable dans $installDir." }
if (-not (Test-Path $shortcut)) { throw "Raccourci du menu Démarrer introuvable : $shortcut." }
if (-not (Test-Path $uninstallKey)) { throw "Entrée de désinstallation absente de HKCU." }
$location = (Get-ItemProperty $uninstallKey).InstallLocation.Trim('"')
Write-Host "Installé dans $location (entrée de désinstallation dans HKCU)."

# 4. Lancement, attente du rendu de « Hello World » (F1/F2), puis fermeture par la croix (F4).
$app = Start-Process -FilePath $exe -PassThru
# Mémorise le handle : sans cela, ExitCode peut rester vide après WaitForExit.
$null = $app.Handle
Wait-Until -TimeoutSeconds 30 -Message "La fenêtre « Arnod » n'est pas apparue." -Condition {
  if ($app.HasExited) { throw "Arnod s'est arrêté au démarrage (code $($app.ExitCode))." }
  $app.Refresh()
  $app.MainWindowTitle -eq 'Arnod'
}
Write-Host "Fenêtre « Arnod » ouverte (PID $($app.Id))."

$failures = @()

# La fenêtre native apparaît avant la WebView : on attend que la page ait rendu le texte
# renvoyé par la commande Rust, lu via l'arbre d'accessibilité (UI Automation) de WebView2.
$started = Get-Date
$rendered = $false
if (Initialize-UIAutomation) {
  try {
    Wait-Until -TimeoutSeconds 60 -Message "« Hello World » n'est pas apparu dans la fenêtre." -Condition {
      if ($app.HasExited) { throw "Arnod s'est arrêté (code $($app.ExitCode))." }
      $null -ne (Find-UIElementByName -Handle $app.MainWindowHandle -Name 'Hello World')
    }
    $rendered = $true
    Write-Host ("« Hello World » affiché après {0:N1} s (F1, F2)." -f ((Get-Date) - $started).TotalSeconds)
  } catch {
    $failures += $_.Exception.Message
  }
} else {
  Write-Host "UI Automation indisponible : attente fixe de 20 s."
  Start-Sleep -Seconds 20
}
Save-Screenshot -Path $Screenshot

if (-not $app.HasExited) {
  if (-not $app.CloseMainWindow()) {
    $failures += "Impossible d'envoyer la fermeture à la fenêtre."
  } elseif (-not $app.WaitForExit(30000)) {
    $app.Refresh()
    Write-Host "Diagnostic : Responding=$($app.Responding), rendu=$rendered."
    Save-Screenshot -Path ($Screenshot -replace '\.png$', '-close.png')
    $failures += "Arnod ne s'est pas fermé dans les 30 s suivant la fermeture de la fenêtre."
  } elseif ($app.ExitCode -ne 0) {
    $failures += "Arnod s'est fermé avec le code $($app.ExitCode)."
  } else {
    Write-Host "Arnod s'est fermé proprement par la croix (code 0, F4)."
  }
}
if (-not $app.HasExited) { Stop-Process -Id $app.Id -Force }
if ($failures.Count -gt 0) { throw ($failures -join ' ') }

# 5. Désinstallation silencieuse.
Start-Process -FilePath $uninstaller -ArgumentList '/S' -Wait | Out-Null
# L'uninstaller NSIS se recopie dans %TEMP% et rend la main aussitôt : on attend son effet.
Wait-Until -TimeoutSeconds 60 -Message "La désinstallation n'a pas supprimé arnod.exe et l'entrée HKCU." -Condition {
  -not (Test-Path $exe) -and -not (Test-Path $uninstallKey)
}
Write-Host "Désinstallation silencieuse réussie."
