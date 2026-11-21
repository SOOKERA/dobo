# Confidentialité

Dobo garde vos notes sur votre PC. Pas de compte, pas de connexion, pas de pistage, et rien de ce que vous écrivez ne quitte votre ordinateur.

## Ce qui reste sur votre PC

Tout ce que vous mettez dans Dobo : notes, tâches, idées, étapes, images, fichiers déposés (Dobo garde un lien vers eux, ils restent où ils sont), vos styles et réglages, et les sauvegardes. Ils sont dans le dossier des données indiqué dans **Paramètres › À propos**, ou, pour les sauvegardes, dans le dossier que vous avez choisi pour elles (voir [Sauvegardes et données](backups-and-data.md)). Un Dobo portable les garde dans le dossier `data` à côté de Dobo.exe, ou dans certains cas dans votre profil utilisateur (voir [Installer et mettre à jour](install-and-update.md#dobo-portable)).

Dobo n’a ni statistiques d’usage, ni rapports de plantage, ni bouton « envoyer un rapport » : il n’envoie jamais rien sur vous ou vos notes.

## Quand Dobo va en ligne

**Paramètres › À propos** le dit en une phrase : « Dobo ne va en ligne que pour chercher le titre et l’image d’une page web que vous déposez, et une fois par jour pour chercher les mises à jour (sauf si vous le désactivez). Vos notes restent sur cet ordinateur. »

Rien ne va en ligne avant que vous acceptiez les conditions d’utilisation de Dobo : dans la fenêtre de Dobo Setup, ou au premier lancement du Dobo portable (voir [Installer et mettre à jour](install-and-update.md#les-conditions-dutilisation-au-premier-lancement)). D’ici là, ni recherche de mises à jour ni carte de lien ; tout le reste marche comme d’habitude.

1. **Les cartes de lien des pages que vous déposez.** Quand la barre de dépôt rapide garde le lien de la page web que vous lisiez, Dobo va chercher cette page une fois, pour afficher son titre et son image sur une petite carte. L’image est enregistrée à côté de vos notes, la carte s’affiche donc hors ligne. Seules cette page et son image sont demandées (l’image peut venir d’un autre site, celui qu’indique la page) : les mots de votre ligne ne sont jamais envoyés. Dobo ne va jamais chercher d’adresse sur votre propre réseau (une box, une imprimante). Cliquez sur **L’enlever** dans la barre et rien n’est cherché.
2. **Les mises à jour.** Une fois par jour, Dobo va chercher `latest.json` sur [github.com/sookera/dobo-releases](https://github.com/sookera/dobo-releases), où ses versions sont publiées, pour voir s’il en existe une nouvelle. Seulement s’il y en a une, il y va chercher aussi `Dobo.exe`. Il indique quelle version de Dobo demande, et rien sur vous ni sur vos notes. Vous pouvez couper cela dans **Paramètres › Mises à jour** (**Chercher les mises à jour automatiquement**) : Dobo ne cherche alors que si vous cliquez sur **Rechercher les mises à jour**.

C’est tout. Rien d’autre ne va en ligne : ni vos notes, ni vos recherches, ni vos réglages.

## Ce que Dobo lit sur votre PC

- **Les Pense-bêtes de Windows, au tout premier lancement.** La première fois que Dobo démarre, tant qu’il n’a encore aucune note, il reprend de lui-même vos notes de Pense-bêtes, sans demander. Il lit la base de données de Pense-bêtes sur ce PC, depuis une copie, et rien ne quitte votre PC. Vos Pense-bêtes ne sont jamais modifiés. Vous pouvez les reprendre de nouveau à tout moment avec **Paramètres › Démarrage › Reprendre les notes de Pense-bêtes (Windows)** (**Importer**) ; les notes déjà sur le tableau ne sont pas ajoutées deux fois.
- **La fenêtre au premier plan, quand la barre de dépôt rapide s’ouvre.** Chaque fois que vous ouvrez la barre (**Ctrl+Maj+D**, ou **+ Déposer** dans le menu de l’icône), Dobo lit le titre de la fenêtre au premier plan. Quand cette fenêtre est un navigateur web, il lit aussi l’adresse de la page dans la barre d’adresse, avec UI Automation de Windows (la fonction d’accessibilité de Windows). Cela reste sur votre PC : ça sert seulement à proposer le lien dans la barre. Seul un lien que vous gardez est demandé, pour son titre et son image (voir plus haut). **L’enlever** le retire.
- **Les fichiers que vous déposez ou collez** sur le tableau.

## Notifications

Les rappels s’affichent avec les notifications de Windows. Elles restent sur votre PC.

## Tout effacer

- **Installée** : désinstallez Dobo depuis **Paramètres › Applications** de Windows avec **Also delete my notes** (supprimer aussi mes notes) activé. Si vous avez choisi un dossier de sauvegardes dans **Paramètres › Sauvegardes**, la désinstallation ne le supprime pas : il est à vous, supprimez-le vous-même si vous voulez qu’il disparaisse.
- **Portable** : supprimez Dobo.exe et son dossier `data`. Si **Paramètres › À propos** indique que ses notes sont dans votre profil utilisateur, elles sont dans `%APPDATA%\com.sookera.ztodo`. Un dossier de sauvegardes que vous avez choisi reste tant que vous ne le supprimez pas.
