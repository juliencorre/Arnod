# Plan - MVP, lot H : réseau et authentification

**Version** 1.0 · **Date** 2026-11-13 · **Statut** Livré · **Base** Develop @1ac7a1b

Plan du lot H de [`plan-finalisation-mvp.md`](plan-finalisation-mvp.md) : préférences réseau du
poste (proxy, autorités de certification), échec réseau expliqué, OAuth 2 par identifiants client
réglé au projet et cookies gardés par environnement. Il respecte les règles de
[`CLAUDE.md`](../CLAUDE.md) et les règles communes R1 à R10 du plan d'ensemble (vocabulaire
inchangé, R4 ; ajouts en fin de liste dans les fichiers partagés, R5 ; aucune permission WebView,
R8).

Source : maquette au 13 novembre 2026, pages 1.6 (« Arnod - Préférences »), 5.1 (« Arnod -
Exécution ») et 3.4 (« Arnod - Authentification et Préférences »).

## 1. Objectif

À la fin de ce plan, Arnod :

- s'ouvre sur ses préférences par Ctrl+, ou par le menu Fichier ; la section « Réseau et
  certificats » règle le proxy (celui du système, aucun ou manuel) et les autorités internes
  (1.6-e) ;
- explique un envoi qui n'aboutit pas : certificat non reconnu, proxy, nom introuvable, serveur
  injoignable, délai ; dit ce qui n'est pas parti et propose l'action qui règle le poste, sans
  jamais couper la vérification TLS (5.1-i) ;
- obtient, garde et renouvelle un jeton OAuth 2 (identifiants client) réglé une fois sur le projet,
  et montre qui l'hérite et qui le remplace (3.4-a) ;
- garde les cookies reçus par environnement, les renvoie au même domaine, les liste et les vide
  (3.4-g).

## 2. Maquettes

| Repère | Écran                                                | Dans ce plan |
| ------ | ---------------------------------------------------- | ------------ |
| 1.6-e  | Préférences → Réseau et Certificats                  | PR1 à PR6    |
| 5.1-i  | Échec réseau : certificat non reconnu (et autres)    | EC1 à EC6    |
| 3.4-a  | Réglages du projet → Authentification (OAuth 2)      | OA1 à OA5    |
| 3.4-g  | Cookies gardés de l'environnement, dans « En-têtes » | CK1 à CK4    |

## 3. Modèle et règles

- **Préférences du poste** : `network.json` dans `app_local_data_dir()` (jamais dans le projet),
  écrit de façon atomique. Mot de passe du proxy au coffre (`arnod/preferences/proxy`).
- **Proxy effectif** : celui du système (variables `HTTPS_PROXY`, `HTTP_PROXY`, `NO_PROXY`, puis,
  sous Windows, `Internet Settings` du registre), aucun, ou manuel (adresse, exceptions,
  identifiant).
- **Autorités** : celles du système (`rustls-native-certs` ; repli sur les racines Mozilla si le
  magasin est vide) plus les autorités internes ajoutées (PEM gardé dans `network.json`).
- **Échec réseau** (`NetworkFailure`) : la cause classée, l'hôte, le proxy utilisé, ce qui n'est
  pas parti et, pour un certificat, le certificat présenté et l'autorité à laquelle faire confiance.
- **Jeton OAuth 2** : par projet, environnement, URL, client et portée ; en mémoire, avec son
  expiration.
- **Cookie gardé** : par projet et environnement ; les cookies persistants sont écrits dans
  `projects/<id>/cookies.json` (poste), ceux de session restent en mémoire.

| ID  | Règle                                                                                                                                                                                     |
| --- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| R1  | Les préférences sont lues une fois, gardées en mémoire ; toute modification est écrite (temporaire puis renommage) et reconstruit le client HTTP. Un fichier illisible donne les défauts. |
| R2  | Proxy « système » : `HTTPS_PROXY` pour https, `HTTP_PROXY` pour http (casse indifférente), sinon le registre Windows (`ProxyEnable`, `ProxyServer`, `ProxyOverride`).                     |
| R3  | Exceptions : `*` tout, `<local>` les noms sans point, `.corp` ou `*.corp` le domaine et ses sous-domaines, sinon le nom exact ou l'adresse IP ; sans tenir compte de la casse.            |
| R4  | La vérification TLS n'est jamais désactivable ; une autorité s'ajoute depuis un PEM lu (sujet, émetteur, validité, empreinte SHA-256 montrés avant) ou depuis l'échec d'un envoi.         |
| R5  | Échec classé : certificat (autorité inconnue, expiré, pour un autre nom), proxy injoignable, proxy qui demande une identification (407), nom introuvable, serveur injoignable, délai.     |
| R6  | Un échec avant la connexion au serveur dit « La requête n'est pas partie » ; un délai dépassé après l'envoi dit qu'elle a pu arriver.                                                     |
| R7  | « Faire confiance » n'est proposé que pour une autorité inconnue dont le certificat a été présenté par le serveur ; sinon, « Ajouter une autorité… » depuis un fichier.                   |
| R8  | OAuth 2 : seul le flux `client_credentials` du schéma est géré (`accessTokenUrl`, `credentials`, `scope`, `tokenConfig.placement`) ; les autres flux restent conservés et non envoyés.    |
| R9  | Le jeton est demandé au premier envoi, gardé jusqu'à 60 s avant son expiration, puis redemandé ; « Renouveler maintenant » et « Oublier le jeton » agissent sur l'environnement choisi.   |
| R10 | Le secret client n'est jamais écrit dans le projet : l'interface le range au coffre (variable locale secrète) et écrit sa référence `{{oauth2ClientSecret}}`.                             |
| R11 | Placement du jeton : en-tête `Authorization` → `Bearer <jeton>` ; autre en-tête → le jeton seul ; paramètre de requête → `?nom=<jeton>`.                                                  |
| R12 | Un cookie reçu (`Set-Cookie`) est gardé pour l'environnement de l'envoi ; il est renvoyé aux requêtes du même domaine (et chemin), sauf si la requête a son propre en-tête `Cookie`.      |
| R13 | Un cookie expiré est oublié ; `Max-Age` prime sur `Expires` ; un `Domain` qui ne couvre pas l'hôte est refusé.                                                                            |
| R14 | Les valeurs des cookies `HttpOnly` sont masquées dans la liste ; les autres sont abrégées.                                                                                                |
| R15 | « Renvoyer les cookies reçus de ce domaine » décoché : les cookies du domaine restent gardés mais ne partent plus.                                                                        |

## 4. Cas d'usage

| ID  | Cas d'usage                    | Écrans | Comportement attendu                                                                                               | Priorité |
| --- | ------------------------------ | ------ | ------------------------------------------------------------------------------------------------------------------ | -------- |
| PR1 | Ouvrir les préférences         | 1.6-e  | Ctrl+, ou Fichier → Préférences… ouvre le dialogue sur « Réseau » ; Échap le ferme.                                | Must     |
| PR2 | Voir le proxy du système       | 1.6-e  | « Celui du système » montre le proxy détecté et sa source, ou « Aucun proxy détecté ».                             | Must     |
| PR3 | Saisir un proxy                | 1.6-e  | « Manuel » : adresse, exceptions, identifiant, mot de passe (coffre) ; enregistré seul.                            | Must     |
| PR4 | Délai de connexion             | 1.6-e  | Connexion en secondes, 10 par défaut.                                                                              | Should   |
| PR5 | Ajouter une autorité           | 1.6-e  | « + Ajouter une autorité… » lit un PEM, montre sujet, validité, empreinte ; « Faire confiance » l'ajoute.          | Must     |
| PR6 | Retirer une autorité           | 1.6-e  | Chaque autorité interne a « Retirer ».                                                                             | Must     |
| EC1 | Certificat non reconnu         | 5.1-i  | Cause, serveur, émetteur, validité, empreinte ; « Faire confiance… » (confirmation) puis renvoyer.                 | Must     |
| EC2 | Certificat expiré ou autre nom | 5.1-i  | Dates ou noms couverts ; pas de « Faire confiance ».                                                               | Must     |
| EC3 | Proxy injoignable ou 407       | 5.1-i  | Proxy utilisé, « Vérifier le proxy… » ouvre les préférences.                                                       | Must     |
| EC4 | Nom introuvable                | 5.1-i  | Le nom cherché et le proxy utilisé, s'il y en a un.                                                                | Must     |
| EC5 | Serveur injoignable, délai     | 5.1-i  | Hôte, port, délai de connexion.                                                                                    | Must     |
| EC6 | Échec partagé                  | 5.1-i  | Le résultat diffusé aux autres fenêtres porte l'échec classé (`RequestResult.failure`).                            | Should   |
| OA1 | Régler OAuth 2 au projet       | 3.4-a  | Menu ⋯ du projet → « Réglages du projet… » → Authentification : type, URL, client, secret, portée, envoi du jeton. | Must     |
| OA2 | Jeton obtenu et renouvelé      | 3.4-a  | Premier envoi : jeton demandé ; ensuite réutilisé ; redemandé avant l'expiration.                                  | Must     |
| OA3 | État du jeton                  | 3.4-a  | « Obtenu à … · expire à … », « Renouveler maintenant », « Oublier le jeton ».                                      | Must     |
| OA4 | Qui l'utilise                  | 3.4-a  | N requêtes l'héritent ; dossiers et requêtes qui la remplacent, avec leur type.                                    | Must     |
| OA5 | Requête : OAuth 2 hérité       | 3.4-b  | L'onglet Auth d'une requête résume l'OAuth 2 hérité ; une requête peut aussi régler son propre OAuth 2.            | Should   |
| CK1 | Cookie gardé et renvoyé        | 3.4-g  | Un `Set-Cookie` est gardé ; la requête suivante vers le domaine l'envoie.                                          | Must     |
| CK2 | Voir les cookies               | 3.4-g  | Onglet « En-têtes » : « Cookies gardés · domaine », nom, valeur, expiration, reçu de.                              | Must     |
| CK3 | Vider                          | 3.4-g  | « Vider » oublie les cookies du domaine pour l'environnement.                                                      | Must     |
| CK4 | Ne plus renvoyer               | 3.4-g  | Case « Renvoyer les cookies reçus de ce domaine ».                                                                 | Should   |

## 5. Hors périmètre

- Configuration automatique du proxy (PAC, WPAD) et identification Windows (NTLM, Kerberos) auprès
  du proxy : signalées, non gérées (V3).
- Autres flux OAuth 2 (code d'autorisation, mot de passe), certificat client : V3 (3.4-d).
- Apparence, raccourcis, mises à jour dans les préférences : sections à venir (1.6-f, 1.6-g, lot L).

## 6. Architecture

### 6.1 Formats sur disque

`network.json` (dossier de données, format 1) :

```json
{
  "format": 1,
  "proxy": {
    "mode": "system",
    "manual": { "url": "http://proxy.corp:8080", "noProxy": "<local>;.corp", "username": "" }
  },
  "connectTimeoutMs": 10000,
  "authorities": [{ "pem": "-----BEGIN CERTIFICATE-----…", "addedAt": 1763030000000 }]
}
```

`projects/<id>/cookies.json` : `{ "<environnement>": { "cookies": [...], "disabledDomains": [] } }`,
cookies persistants seulement. Projet : `request.auth` de `opencollection.yml` au format du schéma
(`type: oauth2`, `flow: client_credentials`), aucun champ inventé.

### 6.2 Cœur Rust

- `src-tauri/src/network/` : `prefs` (préférences, disque), `proxy` (proxy effectif, exceptions,
  registre derrière le trait `SystemProxySource`), `cert` (lecture DER minimale : sujet, émetteur,
  validité, empreinte), `tls` (configuration rustls : racines du système et internes, vérificateur
  qui garde le certificat refusé), `failure` (classement des échecs), `oauth` (jetons), `cookies`
  (magasin), `mod` (`Network` : client reconstruit, envoi).
- `http::send_with` reçoit le client et le classement ; `Workbench` passe par
  `Network::send` (seul changement de `workbench.rs` : l'appel du client).
- `CommandError.failure` (facultatif, absent du JSON s'il est vide) porte `NetworkFailure`.
- Crates : `rustls` (déjà tirée par reqwest), `rustls-native-certs`, `webpki-roots` et `ring` (déjà
  tirées), `cookie` (déjà tirée par Tauri), `windows-registry` (Windows). Les certificats des
  tests sont construits avec `ring` (`network/testcert.rs`) : pas de crate de développement.

### 6.3 Commandes IPC

| Commande                   | Paramètres                                        | Réponse            | Erreurs                |
| -------------------------- | ------------------------------------------------- | ------------------ | ---------------------- |
| `network_load`             | —                                                 | `NetworkView`      | —                      |
| `network_save`             | `prefs: NetworkPrefsInput`                        | `NetworkView`      | `invalidRequest`, `io` |
| `network_pick_authority`   | —                                                 | `CertificateInfo?` | `invalidRequest`, `io` |
| `network_add_authority`    | `pem`                                             | `NetworkView`      | `invalidRequest`, `io` |
| `network_remove_authority` | `fingerprint`                                     | `NetworkView`      | `notFound`, `io`       |
| `project_auth_load`        | `project`, `environment`                          | `ProjectAuthView`  | `notFound`             |
| `project_auth_save`        | `project`, `environment`, `auth`, `clientSecret?` | `ProjectAuthView`  | `invalidRequest`, `io` |
| `oauth2_token_renew`       | `project`, `environment`                          | `ProjectAuthView`  | `network`, `notFound`  |
| `oauth2_token_forget`      | `project`, `environment`                          | `ProjectAuthView`  | —                      |
| `cookies_list`             | `project`, `environment`                          | `CookieJarView`    | `notFound`             |
| `cookies_clear`            | `project`, `environment`, `domain?`               | `CookieJarView`    | `io`                   |
| `cookies_set_sending`      | `project`, `environment`, `domain`, `enabled`     | `CookieJarView`    | `io`                   |

Événement : `arnod://preferences` (menu Fichier → Préférences…) ouvre le dialogue.

### 6.4 Frontend

- Store `network` : préférences, jetons, cookies ; dialogue des préférences ouvert.
- `components/preferences/PreferencesDialog.vue` (navigation des sections, prête pour Apparence,
  Raccourcis, Mises à jour) et `NetworkSection.vue`.
- `components/projects/ProjectSettingsDialog.vue` (onglet Authentification) ; `AuthEditor.vue`
  gère OAuth 2 ; `OAuth2Fields.vue` partagé.
- `components/workbench/NetworkFailurePanel.vue` (5.1-i) dans `ResponsePanel.vue` ;
  `CookiesSection.vue` sous les en-têtes de la requête (3.4-g).

## 7. Tâches

- [x] **T1** — Cœur : préférences, proxy, autorités, client reconstruit, classement des échecs.
- [x] **T2** — Cœur : OAuth 2 (modèle, OpenCollection, jeton), cookies.
- [x] **T3** — IPC, stores, préférences, échec expliqué, réglages du projet, cookies.
- [x] **T4** — Aperçus 1.6-e, 5.1-i, 3.4-a, 3.4-g ; e2e ; `maquette-etat.yml`, `CLAUDE.md`.

## 8. Critères d'acceptation

- [x] `cargo test` : chaque cause d'échec classée sur un serveur local (TLS par `testcert`,
      faux proxy 407, port fermé, nom en `.invalid`, délai) ; jeton obtenu une fois puis renouvelé ;
      cookie renvoyé.
- [x] Vitest : préférences au clavier, « Faire confiance… », réglages OAuth 2, cookies vidés.
- [ ] Sous Windows : proxy du registre et magasin de certificats du système (à vérifier sur un
      poste réel).

## 9. Écarts gardés avec la maquette

- Préférences et réglages du projet en dialogues, pas en onglets (les onglets relèvent du lot F).
- « Celui de Windows » s'intitule « Celui du système » (Linux et Windows) ; « Identification avec
  votre compte Windows » et la configuration automatique ne sont pas gérées (V3).
- Délai « Réponse » non repris : il se règle par requête ou dossier (`settings.http.timeout`).
- La case « Vérifier les certificats des serveurs » est une mention fixe : la vérification n'est
  jamais coupée (R4).
- Pas de ligne `Cookie … automatique` dans les en-têtes : les cookies gardés sont listés juste
  dessous.
