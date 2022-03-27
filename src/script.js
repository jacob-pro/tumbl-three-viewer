$( document ).ready(function() {
    const BLOG_CHOICE = $("#blog-choice")

    // Convert nginx directory index into list of blogs
    function parse_blogs_list(res) {
        const html = $.parseHTML( res );
        const array = Array.from(html[5].children).slice(1)
        const mapped = array.map((e) => e.text.slice(0, -1))
        const filtered = mapped.filter((e) => e !== "Index")
        if (filtered.length < 1) {
            throw new Error("No blogs found")
        }
        BLOG_CHOICE.empty();
        const placeholder = new Option("Choose a blog", "");
        placeholder.setAttribute('disabled', true);
        placeholder.setAttribute('selected', true);
        BLOG_CHOICE.append(placeholder);
        filtered.forEach((d) => BLOG_CHOICE.append(new Option(d,d)));
        BLOG_CHOICE.attr('disabled' , false);
    }

    $.get( "/blogs/" ).then(
        function(res) {
            parse_blogs_list(res);
        },
        function (e) {
            throw new Error("Get /blogs/ failed")
        }
    ).catch((e) => {
        alert(e);
    })

    BLOG_CHOICE.change(function() {
        const blog = $(this).val();
        console.log("Loading blog files: " + blog);
        const requests = [
            Image.load(blog),
            Text.load(blog),
        ]
        $.when(...requests).then((...responses) => {
            console.log(responses.flat())
        }).catch((e) => {
            alert(e);
        })
    });

});

class Post {

    static get_blog_file(blog, file) {
        const path = `/blogs/${blog}/${file}`;
        return $.get(path).catch((res) => {
            if (res.status === 404) {
                return ""
            }
            throw new Error(`Error getting blog file: ${path} code: ${res.status}`)
        })
    }

}

class Image extends Post {

    static load(blog) {
        return Post.get_blog_file(blog, "images.txt").then((images) => {
            return []
        })
    }

}

class Text extends Post {

    static load(blog) {
        return Post.get_blog_file(blog, "texts.txt").then((texts) => {
            return []
        })
    }

}

