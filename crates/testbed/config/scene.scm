;; Testbed Scene Configuration
;; This file configures the physics testbed scene using Steel (Scheme)

;; =============================================================================
;; Ground Configurations
;; =============================================================================

;; Test 1: CSM-style solid cube ground
;; - material: 32 (green-ish color from palette)
;; - size_shift: 3 (2^3 = 8 units cube edge, origin-centered)
(define ground-1
  (make-ground-cube 32 3))

;; Test 2: Simple cuboid ground
;; - Dimensions: 8x8x8 units, origin-centered
(define ground-2
  (make-ground-cuboid 8 8 8))

;; =============================================================================
;; Scene Objects
;; =============================================================================

;; Falling cube object
;; - Position: (0, 6, 0) - 6 units above ground
;; - Rotation: slight rotation for visual interest
(define scene-objects
  (list
    (make-object
      (vec3 0 6 0)
      (quat-euler 0.1 0.2 0.3))))

;; =============================================================================
;; Camera Configuration
;; =============================================================================

;; Camera setup for observing the scene
;; - Position: (0, 6, -3) - eye level, slightly back
;; - Look-at: (0, 0, 4) - looking towards ground center, forward
(define scene-camera
  (make-camera
    (vec3 0 6 -3)
    (vec3 0 0 4)))

;; =============================================================================
;; Complete Scene Definitions
;; =============================================================================

;; Scene 1: Using solid cube ground (CSM-style)
(define scene-1
  (make-scene
    ground-1
    scene-objects
    scene-camera))

;; Scene 2: Using cuboid ground
(define scene-2
  (make-scene
    ground-2
    scene-objects
    scene-camera))

;; Default scene (used if no specific scene is requested)
(define default-scene scene-1)
