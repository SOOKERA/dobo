# Signaler un problème

Quand quelque chose ne va pas dans Dobo, dites-le à Sookera en quelques mots. Rien ne part avant votre clic, et vous voyez d’abord tout ce qui accompagne vos mots.

## Envoyer un rapport

1. Ouvrez **Signaler un problème** : dans le menu **?** en bas à droite du tableau (**F1**), ou avec **Signaler…** dans **Paramètres › Aide**.
2. Dans **Que s’est-il passé ?**, dites ce que vous avez fait, ce que vous attendiez, et ce qui s’est passé à la place.
3. Regardez **Envoyé avec votre rapport**. Tout ce qui accompagne vos mots y figure :
   - la version de Dobo, et s’il est installé (avec Dobo-Setup.exe) ou portable ;
   - votre version de Windows ;
   - la langue de Dobo, et son apparence (claire ou sombre) ;
   - tant que **Joindre les problèmes récents** est activé, les derniers problèmes notés par Dobo (10 au plus), chacun avec son heure et le fuseau horaire de votre PC à ce moment-là.
4. Choisissez comment l’envoyer :
   - **Envoyer le rapport** (ou **Ctrl+Entrée**) : envoie ce qui est affiché au service de rapports de Sookera. Une fois le rapport envoyé, Dobo affiche son numéro (par exemple **numéro 42**) : indiquez-le si vous écrivez à support@sookera.com au sujet du même problème. S’il ne peut pas partir (pas de connexion, trop de rapports depuis votre connexion, ou le service ne répond pas), Dobo le dit et garde vos mots : réessayez plus tard, ou envoyez-le par e-mail. Vous pouvez fermer la page pendant l’envoi : un message dit ensuite s’il est parti.
   - **Envoyer par e-mail** : votre messagerie ouvre un nouveau message à support@sookera.com, déjà rempli. Relisez-le, puis envoyez-le. Si le rapport est trop long pour un seul lien d’e-mail, Dobo met d’abord les détails dans votre presse-papiers, et le message indique où les coller.
   - **Copier les détails** : pour une messagerie dans le navigateur (Gmail, Outlook sur le web). Collez-les dans un nouvel e-mail à support@sookera.com.

**Envoyer le rapport** marche une fois les conditions d’utilisation de Dobo acceptées (voir [Installer et mettre à jour](install-and-update.md#les-conditions-dutilisation-au-premier-lancement)). **Envoyer par e-mail** et **Copier les détails** marchent toujours.

## Où va un rapport

**Envoyer le rapport** passe par le service de rapports de Sookera (reports.sookera.com), qui le range dans un espace privé sur GitHub que seul Sookera peut lire. Les rapports servent uniquement à corriger Dobo. Votre adresse IP n’est pas conservée : pour éviter qu’on inonde le service, il compte les rapports avec un code qui change chaque jour et dont on ne peut pas retrouver votre adresse.

Un rapport n’emporte jamais vos notes, leurs titres, vos images ou vos fichiers, vos liens ni vos réglages.

## Les problèmes récents

Dobo écrit une courte ligne dans deux petits fichiers de son dossier des données (voir [Sauvegardes et données](backups-and-data.md#où-sont-vos-notes)) quand il doit s’arrêter à cause d’une erreur (`crash.log`), ou quand il ne peut pas démarrer (`startup-error.log`). Chaque ligne dit quand (avec le fuseau horaire de votre PC à ce moment-là), quelle version de Dobo, et quelle était l’erreur. Ces fichiers restent sur votre PC : une ligne n’en sort que dans un rapport que vous envoyez vous-même, et seulement tant que **Joindre les problèmes récents** est activé.

Avant qu’une ligne soit écrite, puis de nouveau avant son envoi, Dobo retire ce qui pourrait dire qui vous êtes ou ce que vous avez écrit :

- votre dossier utilisateur Windows devient `%USERPROFILE%`, et votre nom d’utilisateur seul devient `…` ;
- les noms de dossiers et de fichiers deviennent `…`, et un fichier ne garde que son type (`%USERPROFILE%\…\….txt`) ;
- tout ce qui est entre guillemets (droits, courbes ou « », et accents graves) devient `…`, si bien qu’une erreur qui cite les mots d’une note ne les montre jamais ; seuls les noms propres à Rust, comme `Option::unwrap()`, restent ;
- les liens ne gardent que leur site (`https://example.com`) ; les adresses e-mail et les longs codes sont retirés.

Ce qui reste dit où l’erreur s’est produite dans Dobo lui-même (par exemple `src\db.rs:120`).

## Idées et questions

Pour une idée, un souhait ou une question plutôt qu’un problème, **Partagez-la sur GitHub Discussions**, en bas de la page, ouvre [github.com/sookera/dobo/discussions](https://github.com/sookera/dobo/discussions). Ce que vous y écrivez est public.
