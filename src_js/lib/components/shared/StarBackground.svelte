<script lang="ts">
	interface StarPosition {
		x: number;
		y: number;
		rotation: number;
		scale: number;
		opacity: number;
	}

	// Poisson disc sampling for evenly distributed stars
	function poissonDiscSampling(
		width: number,
		height: number,
		minDist: number,
		maxAttempts: number = 30
	): StarPosition[] {
		const cellSize = minDist / Math.sqrt(2);
		const gridWidth = Math.ceil(width / cellSize);
		const gridHeight = Math.ceil(height / cellSize);
		const grid: (StarPosition | null)[][] = Array(gridWidth)
			.fill(null)
			.map(() => Array(gridHeight).fill(null));
		const points: StarPosition[] = [];
		const activeList: StarPosition[] = [];

		// Helper to get grid coords
		const getGridCoords = (x: number, y: number) => ({
			i: Math.floor(x / cellSize),
			j: Math.floor(y / cellSize)
		});

		// Helper to check if point is valid
		const isValidPoint = (p: StarPosition): boolean => {
			const { i, j } = getGridCoords(p.x, p.y);

			// Check neighboring cells
			const startI = Math.max(0, i - 2);
			const endI = Math.min(gridWidth - 1, i + 2);
			const startJ = Math.max(0, j - 2);
			const endJ = Math.min(gridHeight - 1, j + 2);

			for (let gi = startI; gi <= endI; gi++) {
				for (let gj = startJ; gj <= endJ; gj++) {
					const neighbor = grid[gi][gj];
					if (neighbor) {
						const dx = p.x - neighbor.x;
						const dy = p.y - neighbor.y;
						if (Math.sqrt(dx * dx + dy * dy) < minDist) {
							return false;
						}
					}
				}
			}
			return true;
		};

		// Start with initial point
		const initialPoint: StarPosition = {
			x: Math.random() * width,
			y: Math.random() * height,
			rotation: Math.random() * 360,
			scale: 0.0015 + Math.random() * 0.0015,
			opacity: 0.3 + Math.random() * 0.3
		};

		const { i, j } = getGridCoords(initialPoint.x, initialPoint.y);
		grid[i][j] = initialPoint;
		points.push(initialPoint);
		activeList.push(initialPoint);

		while (activeList.length > 0) {
			const randomIndex = Math.floor(Math.random() * activeList.length);
			const point = activeList[randomIndex];
			let found = false;

			for (let k = 0; k < maxAttempts; k++) {
				const angle = Math.random() * Math.PI * 2;
				const radius = minDist + Math.random() * minDist;
				const newPoint: StarPosition = {
					x: point.x + radius * Math.cos(angle),
					y: point.y + radius * Math.sin(angle),
					rotation: Math.random() * 360,
					scale: 0.0015 + Math.random() * 0.0015,
					opacity: 0.3 + Math.random() * 0.3
				};

				if (
					newPoint.x >= 0 &&
					newPoint.x < width &&
					newPoint.y >= 0 &&
					newPoint.y < height &&
					isValidPoint(newPoint)
				) {
					const { i: ni, j: nj } = getGridCoords(newPoint.x, newPoint.y);
					grid[ni][nj] = newPoint;
					points.push(newPoint);
					activeList.push(newPoint);
					found = true;
					break;
				}
			}

			if (!found) {
				activeList.splice(randomIndex, 1);
			}
		}

		return points;
	}

	// Generate stars with minimum distance of 3 units in 100x100 viewBox
	const stars = poissonDiscSampling(100, 100, 3);
</script>

<div class="star-background">
	<svg
		class="star-field"
		viewBox="0 0 100 100"
		xmlns="http://www.w3.org/2000/svg"
		preserveAspectRatio="xMidYMid slice"
	>
		<defs>
			<!-- Define the star shape once (normalized to ~1x1 size) -->
			<path
				id="star-shape"
				d="M180.906,12.469c6.752-16.64,30.312-16.64,37.064,0l44.568,109.817l117.813,8.769c17.823,1.326,25.09,23.581,11.485,35.17l-90.3,76.918l28.11,115.48c4.24,17.416-14.807,31.186-30.016,21.7L199.438,317.836L99.251,380.323c-15.209,9.486-34.256-4.284-30.016-21.7l28.11-115.48l-90.3-76.918c-13.605-11.589-6.338-33.844,11.485-35.17L136.338,122.286Z"
				fill="currentColor"
			/>
		</defs>

		<!-- Generate stars using the defined shape -->
		{#each stars as star (star)}
			<g
				transform="translate({star.x}, {star.y}) scale({star.scale}) rotate({star.rotation})"
				opacity={star.opacity}
			>
				<use href="#star-shape" x="-200" y="-200" />
			</g>
		{/each}
	</svg>
</div>

<style>
	.star-background {
		position: absolute;
		inset: 0;
		pointer-events: none;
		overflow: hidden;
		color: #fbbf24;
		opacity: 1;
	}

	.star-field {
		position: absolute;
		width: 200%;
		height: 200%;
		top: -50%;
		left: -50%;
		animation: scroll-stars 60s linear infinite;
		will-change: transform;
		contain: layout style paint;
	}

	@keyframes scroll-stars {
		from {
			transform: translate(0, 0);
		}
		to {
			transform: translate(-25%, -25%);
		}
	}
</style>
