<script lang="ts">
	import IntroAnimation from '$lib/components/IntroAnimation.svelte';

	let introComplete = $state(false);
</script>

<IntroAnimation bind:visible={introComplete} />

<div class="main-content" class:visible={introComplete}>
	<div class="min-h-screen bg-tw-darkblue flex flex-col items-center justify-center px-4 pb-20">
		<h1
			class="text-6xl sm:text-7xl font-extrabold mb-14 pb-2
               leading-normal
               bg-linear-to-r from-tw-purple to-tw-pink
               bg-clip-text text-transparent
               drop-shadow-[0_0_12px_rgba(0,245,255,0.5)]"
		>
			ImgCat
		</h1>

		<div
			class="w-full max-w-sm bg-white/5 backdrop-blur
             border border-tw-purple/30 rounded-2xl p-8
             flex flex-col gap-5 items-center"
		>
			<p class="text-tw-neon text-xl font-semibold text-center">Welcome</p>
			<p class="text-white/60 text-sm text-center">Share images with the magic of friendship</p>

			<a
				href="/signin"
				class="w-full text-center rounded-lg py-2.5 font-semibold text-white
                   bg-tw-purple hover:bg-tw-pink cursor-pointer
                   transition-colors duration-200
                   focus:outline-none focus:ring-2 focus:ring-tw-neon"
			>
				Sign In
			</a>

			<a
				href="/register"
				class="w-full text-center rounded-lg py-2.5 font-semibold
                   text-tw-neon border border-tw-neon/40
                   hover:bg-tw-neon/10 cursor-pointer
                   transition-colors duration-200
                   focus:outline-none focus:ring-2 focus:ring-tw-neon"
			>
				Register
			</a>
		</div>
	</div>
</div>

<style lang="postcss">
	@reference "tailwindcss";

	:global(html) {
		background-color: #172757;
	}

	.main-content {
		opacity: 0;
		transition: opacity 1s ease-in;
	}

	.main-content.visible {
		opacity: 1;
	}
</style>
