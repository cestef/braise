**recipe** keyword

Defines a new recipe.

**Syntax:**
```braise
recipe \"name\" {
    // recipe body
}
```

**With dependencies:**
```braise
recipe \"name\" -> [\"dep1\", \"dep2\"] {
    // recipe body
}
```