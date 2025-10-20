# Cube Script Model (CSM)


### Octant - child cube index
```
a=000 (x-,y-,z-)  e=100 (x+,y-,z-)
b=001 (x-,y-,z+)  f=101 (x+,y-,z+)
c=010 (x-,y+,z-)  g=110 (x+,y+,z-)
d=011 (x-,y+,z+)  h=111 (x+,y+,z+)
```

## Grammar
```
Model = Epoch+
Epoch = Statement+ ('|' Epoch)?
Statement = '>' Octant+ Cube
Cube = Value
     | '[' Cube{8} ']'  // Exactly 8 children
     | '<' Path?         // Reference from previous epoch (root if no path)
     | Transform Cube
Value = Integer
Path = Octant+  
Octant = [a-h]
Transform = '/' Axis+   // Mirror along axis(es), order doesn't matter
Axis = 'x' | 'y' | 'z'
```

## Examples
```
# First epoch - build tree
>a = [1 2 3 4 5 6 7 8]
>aa = [10 11 12 13 14 15 16 17]
>aaa = 100
>ab = [20 21 22 23 24 25 26 27]

# Second epoch - reference at various depths
| >a = <          # Copy entire previous root
  >b = <a         # Copy previous epoch's 'a' branch
  >c = <aa        # Copy previous epoch's 'aa' node
  >d = <aaa       # Copy value 100 from deep node
  >e = [<ab <ab <ab <ab 0 0 0 0]  # Mix references with literals
  >f = /x <aa     # Mirror the [10 11...] array

```


