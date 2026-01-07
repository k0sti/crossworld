;; Testbed Scene Configuration
;; This file configures the physics testbed scene using Steel (Scheme)

;; =============================================================================
;; Ground Configurations
;; =============================================================================

;; Test 1: CSM-style solid cube ground
;; - material: 32 (green-ish color from palette)
;; - size_shift: 3 (2^3 = 8 units cube edge)
;; - center: (0, -4, 0) - positioned so top face is at Y=0
(define ground-1
  (ground-cube 32 3 (vec3 0 -4 0)))

;; Test 2: Simple cuboid ground
;; - Dimensions: 8 units (width used for cube size)
;; - center: (0, -4, 0) - positioned so top face is at Y=0
(define ground-2
  (ground-cuboid 8 (vec3 0 -4 0)))

;; =============================================================================
;; Scene Objects
;; =============================================================================

;; Multiple falling cube objects for testing
;; Each object has:
;; - Position: (x, y, z) - world position
;; - Rotation: quaternion (x, y, z, w)
;; - Size: (x, y, z) - half-extents for collider
;; - Mass: kg
;; - Material: color index from palette
(define scene-objects
  (list
    ;; Object 0: Center cube, no rotation
    (object
      (vec3 -1 1 0)
      (quat 0.0 0.0 0.0 1.0)
      (vec3 0.4 0.4 0.4)
      1.0
      224)
    ;; Object 1: Left cube, rotated 45 degrees around Y axis
    (object
      (vec3 1 10 0)
      (quat 0.0 0.0 0.3827 0.9239)  ;; ~45 deg Y rotation
      (vec3 0.8 0.3 0.3)
      0.8
      160)
))

;; =============================================================================
;; Camera Configuration
;; =============================================================================

;; Camera setup for observing the scene
;; - Position: (0, 6, -3) - eye level, slightly back
;; - Look-at: (0, 0, 4) - looking towards ground center, forward
(define scene-camera
  (camera
    (vec3 0 6 -3)
    (vec3 0 0 4)))

;; =============================================================================
;; Complete Scene Definitions
;; =============================================================================

;; Scene 1: Using solid cube ground (CSM-style)
(define scene-1
  (scene
    ground-1
    scene-objects
    scene-camera))

;; Scene 2: Using cuboid ground
(define scene-2
  (scene
    ground-2
    scene-objects
    scene-camera))

;; Default scene (used if no specific scene is requested)
(define default-scene scene-1)
