# Conexión con Discord

## Lo que falta fuera del repositorio

Discord RPC de voz requiere una aplicación registrada y permisos de Discord.
No hay una aplicación registrada para Charlita todavía. El código no puede
conceder esos permisos ni demostrar llamadas reales usando únicamente eventos
simulados.

1. Crea la aplicación en https://discord.com/developers/applications.
2. Conserva su **Application ID público**. No compartas su client secret ni
   tokens de cuenta.
3. Solicita a Discord acceso a los scopes `rpc` y `rpc.voice.read`.
4. Configura el redirect que muestra Charlita en **Ajustes**. El predeterminado
   es `http://127.0.0.1:38465/auth/callback`; debe coincidir con el puerto utilizado.
5. La implementación usa un cliente público y PKCE (S256). Discord documenta
   ese modo para account linking de Social SDK. **Su aceptación para los scopes
   RPC de esta aplicación aún debe comprobarse con Discord**. Si Discord no
   permite esa combinación, se debe resolver el flujo de autorización aprobado
   antes de distribuir la conexión públicamente; no se incrusta un secreto en
   el ejecutable.
6. Guarda el ID en Charlita, pulsa **Autorizar Discord** y completa el consentimiento
   en el navegador que abre el sistema. El cliente usado para voz sigue siendo
   exclusivamente Discord de escritorio.

Las credenciales OAuth se guardan en el almacén del sistema operativo. Si ese
almacén no está disponible, solo permanecen en memoria durante la sesión. No
se incluyen en los proyectos, diagnósticos ni logs de Charlita.

## Validación necesaria

Prueba un canal de servidor, una llamada directa y una llamada de grupo con dos
cuentas, eventos de hablar y silencio, mute, entrada/salida, cambio de canal,
canal fijado y reinicio de Discord. Confirma que la conexión se recupera y que
los personajes descansan sin perder la composición ni expresiones manuales.
La prueba de permisos debe incluir una cuenta externa al equipo de desarrollo.

Referencias oficiales:

- RPC y permisos: https://docs.discord.com/developers/topics/rpc
- OAuth2: https://docs.discord.com/developers/topics/oauth2
- Cliente público y PKCE: https://docs.discord.com/developers/discord-social-sdk/development-guides/account-linking
