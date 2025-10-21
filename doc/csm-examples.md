# Cube Script Model (CSM) Examples

## Simple Examples

### Single Voxel
```
>a 100
```
Creates a single voxel with value 100 at position 'a'.

### Array of 8 Voxels
```
>a [1 2 3 4 5 6 7 8]
```
Creates 8 child voxels with different values.

### Nested Structure
```
>a [1 2 3 4 5 6 7 8]
>aa [10 11 12 13 14 15 16 17]
>aaa 100
```
Creates a hierarchical structure with multiple levels.

## Using Epochs and References

### Copy from Previous Epoch
```
>a [1 2 3 4 5 6 7 8]
| >b <a
```
First epoch creates structure at 'a', second epoch copies it to 'b'.

### Mirror Transform
```
>a [1 2 3 4 5 6 7 8]
| >b /x <a
```
Mirror the structure along the X axis.

### Complex Example
```
# First epoch - build base structure
>a [1 2 3 4 5 6 7 8]
>aa [10 11 12 13 14 15 16 17]
>ab [20 21 22 23 24 25 26 27]

# Second epoch - create variations
| >b <a
  >c /x <a
  >d /y <a
  >e /z <a
  >f /xy <a
```

## Humanoid Character Example
```
# Head
>d [100 100 100 100 100 100 100 100]
>dd [150 150 150 150 150 150 150 150]

# Body/Torso
>c [80 80 80 80 80 80 80 80]
>cd [90 90 90 90 90 90 90 90]

# Arms
>cf [70 70 70 70 0 0 0 0]
>cg [70 70 70 70 0 0 0 0]

# Legs
>a [60 60 0 0 0 0 0 0]
>b [60 60 0 0 0 0 0 0]
```

## Color Values
Different integer values produce different colors (using HSV color mapping):
- 0: Empty (no voxel)
- 1-360: Maps to hue (0=red, 120=green, 240=blue, etc.)
- Negative values: Red color

Try different values to experiment with colors!
