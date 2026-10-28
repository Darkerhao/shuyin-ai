!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    ; The running app recreates the host manifest after an in-place update.
    DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\com.shuyin.desktop.audio"
    DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\com.shuyin.desktop.audio.dev"
  ${EndIf}
!macroend
