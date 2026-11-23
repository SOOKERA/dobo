# Confidentialité

Dobo garde vos notes sur votre PC. Pas de compte, pas de connexion, pas de pistage, et rien de ce que vous écrivez ne quitte votre ordinateur.

## Ce qui reste sur votre PC

Tout ce que vous mettez dans Dobo : notes, tâches, idées, étapes, images, fichiers déposés (Dobo garde un lien vers eux, ils restent où ils sont), vos styles et réglages, et les sauvegardes. Ils sont dans le dossier des données indiqué dans **Paramètres › À propos**, ou, pour les sauvegardes, dans le dossier que vous avez choisi pour elles (voir [Sauvegardes et données](backups-and-data.md)). Un Dobo portable les garde dans le dossier `data` à côté de Dobo.exe, ou dans certains cas dans votre profil utilisateur (voir [Installer et mettre à jour](install-and-update.md#dobo-portable)).

Dobo n’a ni statistiques d’usage, ni rapports de plantage automatiques : il n’envoie jamais rien de lui-même sur vous ou vos notes. Un rapport de problème ne part que si vous l’envoyez (voir plus bas).

## Quand Dobo va en ligne

**Paramètres › À propos** le dit en une phrase : « Dobo ne va en ligne que pour chercher le titre et l’image d’une page web que vous déposez, une fois par jour pour chercher les mises à jour (sauf si vous le désactivez), et pour envoyer un rapport de problème quand vous le demandez. Vos notes restent sur cet ordinateur. »

Rien ne va en ligne avant que vous n’acceptiez les conditions d’utilisation de Dobo : dans la fenêtre d’installation de Dobo, ou au premier lancement du Dobo portable (voir [Installer et mettre à jour](install-and-update.md#les-conditions-dutilisation-au-premier-lancement)). D’ici là, ni recherche de mises à jour ni carte de lien ; tout le reste marche comme d’habitude.

1. **Les cartes de lien des pages que vous déposez.** Quand la barre de dépôt rapide garde le lien de la page web que vous lisiez, Dobo va chercher cette page une fois, pour afficher son titre et son image sur une petite carte. L’image est enregistrée à côté de vos notes, la carte s’affiche donc hors ligne. Seules cette page et son image sont demandées (l’image peut venir d’un autre site, celui qu’indique la page) : les mots de votre ligne ne sont jamais envoyés. Dobo ne va jamais chercher d’adresse sur votre propre réseau (une box, une imprimante). Cliquez sur **L’enlever** dans la barre et rien n’est cherché.
2. **Les mises à jour.** Une fois par jour, Dobo va chercher `latest.json` sur [github.com/sookera/dobo-releases](https://github.com/sookera/dobo-releases), où ses versions sont publiées, pour voir s’il en existe une nouvelle. Seulement s’il y en a une, il y va chercher aussi `Dobo.exe`. Il indique quelle version de Dobo demande, et rien sur vous ni sur vos notes. Vous pouvez couper cela dans **Paramètres › Mises à jour** (**Chercher les mises à jour automatiquement**) : Dobo ne cherche alors que si vous cliquez sur **Rechercher les mises à jour**.

3. **Un rapport de problème, quand vous en envoyez un.** **Signaler un problème** (dans le menu **?**, ou **Paramètres › Aide**) n’envoie rien avant votre clic sur **Envoyer le rapport**, et montre d’abord tout ce qui accompagne vos mots : la version de Dobo, installé ou portable, votre version de Windows, la langue et l’apparence de Dobo et, sauf si vous les désactivez, les derniers problèmes notés par Dobo, sans votre dossier utilisateur, sans noms de dossiers et de fichiers ni ce qui est entre guillemets, et avec les liens réduits à leur site (chacun avec son heure et le fuseau horaire de votre PC à ce moment-là). Il va au service de rapports de Sookera (reports.sookera.com), qui le garde dans un espace privé sur GitHub, uniquement pour corriger Dobo. Votre adresse IP n’est pas conservée. Jamais vos notes, titres, images, fichiers, liens ni réglages. Voir [Signaler un problème](report-a-problem.md).

C’est tout. Rien d’autre ne va en ligne : ni vos notes, ni vos recherches, ni vos réglages.

## Ce que Dobo lit sur votre PC

- **Les Pense-bêtes de Windows, quand vous le demandez.** Au premier lancement, Dobo regarde si Pense-bêtes contient des notes, pour proposer de les récupérer : il lit la base de données de Pense-bêtes sur ce PC, depuis une copie, et se contente de les compter. Elles n’arrivent que si vous cliquez sur **Importer**, dans l’accueil ou plus tard dans **Paramètres › Démarrage › Récupérer les notes de Pense-bêtes (Windows)** ; les notes déjà sur le tableau ne sont pas ajoutées deux fois. Rien ne quitte votre PC, et vos Pense-bêtes ne sont jamais modifiés.
- **La fenêtre au premier plan, quand la barre de dépôt rapide s’ouvre.** Chaque fois que vous ouvrez la barre (**Ctrl+Maj+D**, ou **+ Déposer** dans le menu de l’icône), Dobo lit le titre de la fenêtre au premier plan. Quand cette fenêtre est un navigateur web, il lit aussi l’adresse de la page dans la barre d’adresse, avec UI Automation de Windows (la fonction d’accessibilité de Windows). Cela reste sur votre PC : ça sert seulement à proposer le lien dans la barre. Seul un lien que vous gardez est demandé, pour son titre et son image (voir plus haut). **L’enlever** le retire.
- **Les fichiers que vous déposez ou collez** sur le tableau.

## Les fichiers d’un autre ordinateur

Une carte de fichier peut pointer vers un fichier d’un autre ordinateur : un dossier partagé, un NAS, ou vos Documents ou vos Images quand ils sont sur un serveur. Ouvrir un tel fichier envoie votre identification Windows à cet ordinateur, comme le fait l’Explorateur. Alors, la première fois, la carte demande, une fois par ordinateur : « Les fichiers de nas s’ouvrent depuis le réseau. Autoriser ? » (**Autoriser** / **Pas maintenant**). Rien n’est touché sur cet ordinateur avant que vous l’autorisiez. Dobo retient les ordinateurs autorisés avec vos paramètres ; une sauvegarde restaurée n’apporte jamais les siens. Les programmes et les scripts ne sont jamais lancés depuis une carte, où qu’ils soient.

## Notifications

Les rappels s’affichent avec les notifications de Windows. Elles restent sur votre PC.

## Tout effacer

- **Installée** : désinstallez Dobo depuis **Paramètres › Applications** de Windows avec **Supprimer aussi mes notes** activé. Si vous avez choisi un dossier de sauvegardes dans **Paramètres › Sauvegardes**, la désinstallation ne le supprime pas : il est à vous, supprimez-le vous-même si vous voulez qu’il disparaisse.
- **Portable** : supprimez Dobo.exe et son dossier `data`. Si **Paramètres › À propos** indique que ses notes sont dans votre profil utilisateur, elles sont dans `%APPDATA%\com.sookera.ztodo` (un Dobo portable qui utilise ce dossier y note son propre chemin, dans `.dobo-portable-copies`, pour que la désinstallation d’un Dobo installé garde ces notes). Un dossier de sauvegardes que vous avez choisi reste tant que vous ne le supprimez pas.
