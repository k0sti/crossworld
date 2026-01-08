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

;; Helper function to generate random cube parameters deterministically
;; Returns (value next-seed) tuple
(define (gen-random-cube seed index)
  (let* (
    ;; Generate position X (-3 to 3)
    (px-result (rand-range seed -3.0 3.0))
    (px (car px-result))
    (seed1 (car (cdr px-result)))

    ;; Generate position Y (2 to 10)
    (py-result (rand-range seed1 2.0 10.0))
    (py (car py-result))
    (seed2 (car (cdr py-result)))

    ;; Generate position Z (-3 to 3)
    (pz-result (rand-range seed2 -3.0 3.0))
    (pz (car pz-result))
    (seed3 (car (cdr pz-result)))

    ;; Generate rotation quaternion components
    (rx-result (rand-range seed3 -0.5 0.5))
    (rx (car rx-result))
    (seed4 (car (cdr rx-result)))

    (ry-result (rand-range seed4 -0.5 0.5))
    (ry (car ry-result))
    (seed5 (car (cdr ry-result)))

    (rz-result (rand-range seed5 -0.5 0.5))
    (rz (car rz-result))
    (seed6 (car (cdr rz-result)))

    ;; Compute w to normalize quaternion (w = sqrt(1 - x^2 - y^2 - z^2))
    (rw-sq (- 1.0 (+ (* rx rx) (* ry ry) (* rz rz))))
    (rw (if (> rw-sq 0.0) (sqrt rw-sq) 0.0))

    ;; Generate size (0.2 to 0.6 for each component)
    (sx-result (rand-range seed6 0.2 0.6))
    (sx (car sx-result))
    (seed7 (car (cdr sx-result)))

    (sy-result (rand-range seed7 0.2 0.6))
    (sy (car sy-result))
    (seed8 (car (cdr sy-result)))

    (sz-result (rand-range seed8 0.2 0.6))
    (sz (car sz-result))
    (seed9 (car (cdr sz-result)))

    ;; Generate mass (0.5 to 2.0)
    (mass-result (rand-range seed9 0.5 2.0))
    (mass (car mass-result))
    (seed10 (car (cdr mass-result)))

    ;; Generate material (64 to 224)
    (mat-result (rand-range seed10 64.0 224.0))
    (material (inexact->exact (floor (car mat-result))))
    (next-seed (car (cdr mat-result)))
  )
  (list
    (object
      (vec3 px py pz)
      (quat rx ry rz rw)
      (vec3 sx sy sz)
      mass
      material)
    next-seed)))

;; Generate 10 random cubes with deterministic seed (42)
(define (generate-cubes n seed)
  (define (gen-loop count current-seed acc)
    (if (= count 0)
      acc
      (let* ((result (gen-random-cube current-seed count))
             (cube-obj (car result))
             (next-seed (car (cdr result))))
        (gen-loop (- count 1) next-seed (cons cube-obj acc)))))
  (gen-loop n seed '()))

;; Generate 10 random cubes with seed 42
(define scene-objects
  (generate-cubes 10 42))

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
