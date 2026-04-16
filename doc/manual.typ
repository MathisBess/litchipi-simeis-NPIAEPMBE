#let col-pink = rgb("#e34967")
#let col-blue = rgb("#6A82FB")

#set page(
  margin: (y: 40pt, left: 40pt, right: 20pt),
  numbering: "1 /1",
  number-align: right,
)
#set text(14pt)

#show raw: it => box(
    fill: rgb("#F0F0F0"),
    radius: 3pt,
    inset: 3pt,
    baseline: 3pt,
    text(fill: col-blue, size: 10pt, smallcaps(it))
  )
#show link: it => underline(text(weight: "medium", fill: col-pink, size: 13pt, it))
#show ref: it => {
  let el = it.element
  link(el.location())[#it]
}

#set heading(numbering: "1.")
#show heading.where(level: 1): it => block(width: 100%)[
  #v(20pt)
  #set align(left)
  #set text(25pt, fill: col-pink, weight: "bold")
  #h(-15pt)
  #smallcaps[#counter(heading).display() #it.body]
]
#show heading.where(level: 2): it => block(width: 100%)[
  #set align(left)
  #set text(18pt, weight: "bold")
  #h(-10pt)
  #smallcaps[#counter(heading).display() #it.body]
  #v(5pt)
]
// #show selector(heading.where(level: 1)) : set heading(numbering: none)

#show ref: it => underline(text(fill: col-pink, it))

#align(center, text(50pt, weight: "regular", smallcaps("Simeis")))

Simeis est un jeu par API (inspiré de #link("https://spacetraders.io/")[SpaceTraders]),
dont le but est de faire fructifier votre empire économique dans toute la galaxie.

Dans ce manuel, vous trouverez les mécaniques de base du jeu, c'est à vous de porter
ces mécaniques de bases vers l'excellence intergalactique !

#v(20pt)

Pour avoir le détails des API, leurs paramètres et fonctionnement,
lire le fichier `swagger.json` contenu dans ce dossier
