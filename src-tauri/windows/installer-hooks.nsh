!macro SHUYIN_REMOVE_OLD_AUDIO_HOST HOST
  ReadRegStr $0 HKCU "Software\Google\Chrome\NativeMessagingHosts\${HOST}" ""
  ${If} $0 == "$LOCALAPPDATA\com.shuyin.desktop\audio-bridge\hosts\${HOST}.json"
    DeleteRegValue HKCU "Software\Google\Chrome\NativeMessagingHosts\${HOST}" ""
    DeleteRegKey /ifempty HKCU "Software\Google\Chrome\NativeMessagingHosts\${HOST}"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro SHUYIN_REMOVE_OLD_AUDIO_HOST "com.shuyin.desktop.audio"
  !insertmacro SHUYIN_REMOVE_OLD_AUDIO_HOST "com.shuyin.desktop.audio.dev"
  Delete "$INSTDIR\browser-bridge.exe"
  Delete "$INSTDIR\browser-extension\manifest.json"
  Delete "$INSTDIR\browser-extension\identity.json"
  Delete "$INSTDIR\browser-extension\background.js"
  Delete "$INSTDIR\browser-extension\background-core.js"
  Delete "$INSTDIR\browser-extension\content.js"
  Delete "$INSTDIR\browser-extension\content-core.js"
  Delete "$INSTDIR\browser-extension\popup.js"
  Delete "$INSTDIR\browser-extension\popup.html"
  Delete "$INSTDIR\browser-extension\popup.css"
  RMDir "$INSTDIR\browser-extension"
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    !insertmacro SHUYIN_REMOVE_OLD_AUDIO_HOST "com.shuyin.desktop.audio"
    !insertmacro SHUYIN_REMOVE_OLD_AUDIO_HOST "com.shuyin.desktop.audio.dev"
  ${EndIf}
!macroend
