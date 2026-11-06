#!/usr/bin/env bash
# Test de fumée de l'AppImage Linux sous un serveur X virtuel (Xvfb) :
#   1. l'application démarre et reste vivante ;
#   2. une capture d'écran est enregistrée (vérification visuelle du premier lancement) ;
#   3. Ctrl+Q (menu Fichier > Quitter) ferme l'application avec le code 0 (F4).
#
# Usage : scripts/smoke-linux.sh [chemin/vers/Arnod.AppImage] [capture.png]
# Dépendances : xvfb, xdotool, imagemagick.
set -euo pipefail

appimage="${1:-$(ls src-tauri/target/release/bundle/appimage/*.AppImage | head -n 1)}"
screenshot="${2:-smoke-linux.png}"
display=":${SMOKE_DISPLAY:-99}"

chmod +x "$appimage"

Xvfb "$display" -screen 0 1280x800x24 -nolisten tcp &
xvfb_pid=$!
trap 'kill "$xvfb_pid" 2>/dev/null || true' EXIT
export DISPLAY="$display"
sleep 1

# Pas de FUSE sur les runners CI : l'AppImage s'extrait dans un dossier temporaire.
export APPIMAGE_EXTRACT_AND_RUN=1

"$appimage" &
app_pid=$!

# Attendre l'apparition de la fenêtre « Arnod ».
window=""
for _ in $(seq 1 60); do
  if ! kill -0 "$app_pid" 2>/dev/null; then
    wait "$app_pid" && code=0 || code=$?
    echo "Échec : Arnod s'est arrêté au démarrage (code $code)." >&2
    exit 1
  fi
  # --onlyvisible : GTK crée aussi une fenêtre X non mappée portant le même titre.
  window="$(xdotool search --onlyvisible --name '^Arnod$' 2>/dev/null | head -n 1 || true)"
  [[ -n "$window" ]] && break
  sleep 0.5
done
if [[ -z "$window" ]]; then
  echo "Échec : aucune fenêtre « Arnod » après 30 s." >&2
  kill "$app_pid" 2>/dev/null || true
  exit 1
fi
echo "Fenêtre « Arnod » détectée (id $window)."

# Laisser la WebView charger la page et appeler la commande `hello`.
sleep 3
import -window root "$screenshot"
echo "Capture enregistrée : $screenshot"

# F4 : fermeture via le raccourci du menu Fichier > Quitter.
# Pas de gestionnaire de fenêtres sous Xvfb : on donne le focus directement.
xdotool windowfocus --sync "$window"
xdotool key --clearmodifiers ctrl+q

for _ in $(seq 1 20); do
  if ! kill -0 "$app_pid" 2>/dev/null; then
    wait "$app_pid" && code=0 || code=$?
    if [[ "$code" -ne 0 ]]; then
      echo "Échec : Arnod s'est fermé avec le code $code." >&2
      exit 1
    fi
    echo "Arnod s'est fermé proprement via Ctrl+Q (code 0)."
    exit 0
  fi
  sleep 0.5
done

echo "Échec : Arnod ne s'est pas fermé après Ctrl+Q." >&2
kill "$app_pid" 2>/dev/null || true
exit 1
