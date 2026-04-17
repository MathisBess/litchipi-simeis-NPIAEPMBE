# Rapport CI/CD 
## Nathan PIVETEAU, Adam EPIARD et Mathis BESSON

### TP1 - Mise en place de la ci du projet

#### Les ajouts

- Script automatisé pour les dépendances qui crée une pull request quand elle n'existe pas --> à la base on ne vérifiait pas son existence. Pendant la nuit on a donc observé des erreurs étant donné qu'il essayait de créer une nouvelle en doublon. 
- On a séparé les modèles pour les rapports de bugs et les nouvelles fonctionnalités, car pour nous il est important de différencier ces deux types de retours. Cela permet aussi d'aider à définir leur niveau de priorité.
- Notre bot automatisé vérifie les dépendances à intervalles réguliers, mais cela ne garantit pas qu'une nouvelle dépendance ajoutée soit exempte de faille. C'est pourquoi on a ajouté un audit de sécurité automatique.

#### Ce qu'on a pu apprendre

On était déjà très familiers avec la CI, mais en s'interdisant d'utiliser les outils du marketplace, on a pu découvrir de nouvelles manières de faire et de nouveaux outils comme typst.

#### Amélioration à creuser

- Notre ci est très lente car elle recompile à chaque fois. On a commencé à se renseigner sur l'utilisation du cache pour gagner du temps.
- On aimerait automatiser la création des "Releases" sur GitHub pour que le binaire et le manuel soient téléchargeables en un clic dès qu'on sort une version.

