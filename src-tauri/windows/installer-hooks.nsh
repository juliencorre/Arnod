; Crochets de l'installeur NSIS (V2, lot D, Q3 de docs/plan-v2.md) : la ligne de commande
; `arnod` est livrée dans `$INSTDIR\bin\arnod.exe` ; l'installation ajoute ce dossier au PATH de
; l'utilisateur (HKCU, sans élévation), la désinstallation l'en retire. Inclus par
; tauri.cli.conf.json, seulement pour l'installeur Windows de la CI.
;
; Le PATH n'est jamais tronqué : une valeur trop longue pour NSIS (NSIS_MAX_STRLEN) est laissée
; telle quelle, et la commande reste appelable par son chemin complet.

!include LogicLib.nsh
!include WinMessages.nsh

; Position de $R2 dans $R1 (sans casse, comme StrCmp), ou -1, dans $R0.
!macro ARNOD_STRPOS_FUNCTION PREFIX
  Function ${PREFIX}ArnodStrPos
    Push $R3
    Push $R4
    Push $R5
    StrLen $R3 $R2
    StrCpy $R4 0
    arnod_loop:
      StrCpy $R5 $R1 $R3 $R4
      StrCmp $R5 "" arnod_absent
      StrCmp $R5 $R2 arnod_found
      IntOp $R4 $R4 + 1
      Goto arnod_loop
    arnod_found:
      StrCpy $R0 $R4
      Goto arnod_done
    arnod_absent:
      StrCpy $R0 -1
    arnod_done:
    Pop $R5
    Pop $R4
    Pop $R3
  FunctionEnd
!macroend

!insertmacro ARNOD_STRPOS_FUNCTION ""
!insertmacro ARNOD_STRPOS_FUNCTION "un."

; $R0 = 1 si la valeur Path existe dans HKCU\Environment, sinon 0 (installation seulement).
Function ArnodPathExists
  Push $R8
  Push $R9
    StrCpy $R0 0
    StrCpy $R9 0
    arnod_enum:
      EnumRegValue $R8 HKCU "Environment" $R9
      StrCmp $R8 "" arnod_enum_done
      StrCmp $R8 "Path" 0 +3
        StrCpy $R0 1
        Goto arnod_enum_done
      IntOp $R9 $R9 + 1
      Goto arnod_enum
    arnod_enum_done:
    Pop $R9
    Pop $R8
FunctionEnd

!macro NSIS_HOOK_POSTINSTALL
  ClearErrors
  ReadRegStr $R6 HKCU "Environment" "Path"
  ${If} ${Errors}
    ; Absente : le PATH de l'utilisateur ne contient que la commande ; illisible (trop longue
    ; pour NSIS) : elle n'est pas touchée.
    Call ArnodPathExists
    ${If} $R0 == 0
      WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR\bin"
      SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
    ${Else}
      DetailPrint "PATH trop long : ajoutez $INSTDIR\bin à la main pour la commande arnod."
    ${EndIf}
  ${Else}
    StrCpy $R1 ";$R6;"
    StrCpy $R2 ";$INSTDIR\bin;"
    Call ArnodStrPos
    ${If} $R0 == -1
      StrLen $R7 "$R6;$INSTDIR\bin"
      IntOp $R7 $R7 + 1
      ${If} $R7 < ${NSIS_MAX_STRLEN}
        ${If} $R6 == ""
          WriteRegExpandStr HKCU "Environment" "Path" "$INSTDIR\bin"
        ${Else}
          WriteRegExpandStr HKCU "Environment" "Path" "$R6;$INSTDIR\bin"
        ${EndIf}
        SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
      ${Else}
        DetailPrint "PATH trop long : ajoutez $INSTDIR\bin à la main pour la commande arnod."
      ${EndIf}
    ${EndIf}
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ClearErrors
  ReadRegStr $R6 HKCU "Environment" "Path"
  ${IfNot} ${Errors}
    StrCpy $R1 ";$R6;"
    StrCpy $R2 ";$INSTDIR\bin;"
    Call un.ArnodStrPos
    ${If} $R0 != -1
      ; Avant et après l'entrée retirée, puis sans les deux « ; » ajoutés pour la recherche.
      StrCpy $R3 $R1 $R0
      StrLen $R4 $R2
      IntOp $R4 $R4 + $R0
      StrCpy $R5 $R1 "" $R4
      StrCpy $R7 "$R3;$R5" "" 1
      StrCpy $R7 $R7 -1
      ${If} $R7 == ""
        DeleteRegValue HKCU "Environment" "Path"
      ${Else}
        WriteRegExpandStr HKCU "Environment" "Path" $R7
      ${EndIf}
      SendMessage ${HWND_BROADCAST} ${WM_SETTINGCHANGE} 0 "STR:Environment" /TIMEOUT=5000
    ${EndIf}
  ${EndIf}
!macroend
