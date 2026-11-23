# Heures et rappels

Écrivez une heure comme vous la diriez, et Dobo vous le rappelle. Finissez une ligne par **!** et elle devient une échéance, qui commence tôt et insiste un peu plus à l’approche.

## L’heure en toutes lettres

Écrivez-la dans la ligne, en français ou en anglais, quelle que soit la langue de Dobo : « Appeler Anna demain 15h », « dentiste vendredi 9h », « rapport lundi 14h30 », « 15h », « tomorrow 9am ».

- La ligne sous le champ montre comment Dobo l’a lue avant que vous n’appuyiez sur **Entrée** : « Rappel à l’heure dite · finissez par **!** pour une échéance » (dans une fenêtre étroite, simplement « Échéance : finissez par **!** »).
- L’heure est retirée de vos mots et s’affiche en petite pastille sur la tâche.
- Un jour sans heure (« demain ») veut dire 09:00. Un moment de la journée garde son heure : « ce soir », « demain soir », « cet après-midi », « tonight » (celui d’aujourd’hui, même une fois commencé).
- « à 3 » tout seul (sans « h », sans minutes) veut dire 15:00, de 1 à 6. Avec un « h », c’est l’heure écrite : « à 3h », c’est 03:00.
- « midi », « minuit » et « après-demain » marchent aussi.
- Les idées n’ont pas d’heure. Faites-en d’abord une tâche si vous en voulez une.

## Une échéance : finissez par **!**

« Envoyer le rapport vendredi 17h! » est une échéance. L’aide affiche « Échéance : sonne en avance, puis de plus en plus souvent », et sa pastille porte un petit sablier.

Une échéance suit un **rythme**. **Normal**, celui de départ, commence 60 minutes avant, sonne encore à 30, 15 et 5 minutes avant, puis toutes les 5 minutes après l’heure, 6 fois au plus. **Doux** est plus calme, **Insistant** plus fort, et **Sur mesure** est le vôtre.

- Changez celui par défaut dans **Paramètres › Rappels › Rythme par défaut des échéances**.
- Changez-le pour une tâche : son menu › **Type de rappel** : **Sonne à l’heure** ou **Avec un rythme**, et les minutes du rythme.

Un simple rappel sonne à l’heure, puis toutes les 5 minutes jusqu’à votre réponse, 6 fois au plus.

## Donner ou changer une heure plus tard

- **Le menu de la tâche** (**⋯**) : tapez une **Nouvelle heure** (« demain 10h », « vendredi 15h ») et cliquez sur **Valider**, ou **Retirer l’heure**.
- **Les heures du jour** : glissez la tâche sur une heure, ou **Choisir sur les heures du jour** dans son menu (voir [Les heures du jour](the-hours-ruler.md)).
- **Trier** : le champ **Quand ?**.
- **En modifiant la ligne** : voir « Dates écrites dans une note » dans [Tâches et idées](tasks-and-ideas.md).

## Les répétitions

Écrivez-la dans la ligne : « arroser les plantes tous les lundis », « sport tous les lundis et jeudis 19h », « point d’équipe en semaine 9h30 », « payer le loyer chaque 1er du mois », « toutes les 2 semaines », « tous les jours 8h ». La première est fixée au prochain jour qui correspond, et la pastille montre un signe de répétition.

Ou passez par le menu de la tâche › **Répéter** : **Jamais**, **Jour**, **Lun–ven**, **Semaine** (choisissez les jours) ou **Mois** (un **Jour du mois** : le 29, le 30 ou le 31 tombe le dernier jour des mois plus courts) ; **Tous les combien** ; et la **Fin** : **Jamais**, **Après** un certain nombre de fois, ou **À une date** (son **Dernier jour**). Cocher une tâche qui se répète crée la suivante, à la même heure. Cochée en retard, elle passe à la prochaine encore à venir.

Une répétition garde son heure quand on change d’heure : une répétition à 02:30 sonne de nouveau à 02:30 après le passage à l’heure d’été. La nuit où 02:30 n’existe pas, elle sonne à 03:00, la première minute qui suit ; la nuit où 02:30 arrive deux fois, elle sonne à la première.

## Répondre à un rappel

Un rappel arrive en notification Windows, et s’affiche aussi sur la **carte du moment**, en haut de Dobo. Répondez à l’une ou l’autre :

- **Fait** coche la tâche.
- **10 min** (ou **1 h**, pour un simple rappel) sonnera de nouveau plus tard.
- **Plus tard…** ouvre Dobo avec d’autres choix : **Dans 1 heure**, **Ce soir**, **Demain matin**, **Lundi prochain**, ou **Choisir une heure** (« demain 14h », « vendredi 9h »).
- **Je m’en occupe** (une échéance) : Dobo arrête de sonner, et un peu plus tard demande « « Envoyer le rapport », vraiment fini ? », avec **Oui, fini** et **Pas encore**.

Quand plusieurs sonnent en même temps (ou sont arrivés à l’heure pendant la veille de l’ordinateur), Windows les montre dans une seule notification : « 3 rappels », avec les mots des premiers et **Ouvrir Dobo**. Répondez à chacun sur la carte du moment, **10 min** et **Plus tard…** compris : elle affiche **Encore 2**, cliquez pour voir le suivant. Un rappel qui sonne seul garde sa propre notification et ses boutons.

Décocher une tâche dont l’heure est passée (ou annuler **Fait**) attend une minute avant de la faire sonner, pour qu’elle ne sonne jamais à l’instant où vous changez d’avis.

« Ce soir » et « Demain matin » suivent **Paramètres › Ma journée** (de 08:00 à 22:00 au départ). Encore debout après minuit, « Demain matin » est le matin qui vient, dans quelques heures.

## Les étapes

Une étape qui a une heure sonne comme n’importe quelle tâche. Cochez la tâche dont elle fait partie, et ses étapes cessent de sonner ; leurs heures sont gardées. Décochez la tâche, et elles sonnent de nouveau, sauf une étape dont l’heure est arrivée pendant que la tâche était faite : elle reste en sourdine jusqu’à ce que vous lui donniez une nouvelle heure. Voir [Les étapes](steps.md).

## 12 ou 24 heures

Dobo affiche les heures comme Windows : « 15:00 », ou « 3:00 PM » quand Windows est réglé sur 12 heures (**Paramètres de Windows › Heure et langue › Langue et région › Format régional**). Cela vaut pour le tableau, la carte du moment, les heures du jour, les pastilles, les Paramètres, le bilan et les notifications, et suit tout de suite un changement dans Windows. Les mots restent dans la langue de Dobo. Les heures tapées marchent pareil dans les deux cas : « 15h », « 3pm », « demain 9h ».

## Les rappels ne sonnent que si Dobo tourne

Fermer la fenêtre laisse Dobo dans la zone de notification, et les rappels sonnent toujours. **Quitter Dobo** les arrête. Laissez **Paramètres › Démarrage › Démarrer Dobo avec Windows** activé pour qu’il soit toujours là, et autorisez les notifications de Dobo dans Windows.

Si l’ordinateur était en veille ou éteint à l’heure dite, le rappel sonne une fois au retour de Dobo (pas une fois par sonnerie manquée), et dit quand il était prévu : « Prévu hier à 22:00 ». Quand plusieurs étaient prévus entre-temps, ils arrivent ensemble dans une seule notification.

Un rappel de l’autre espace (Travail ou Perso) sonne quand même : le tableau y passe le temps de ce rappel, avec un bouton pour revenir. Les rappels d’une note archivée restent en sourdine jusqu’à ce que vous la restauriez.
